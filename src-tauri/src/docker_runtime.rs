use crate::project_catalog::{ComposeExecutionContext, DiscoveredProject, ProjectCatalog};
use crate::runtime::{
    ApplicationSnapshot, ConnectionStatus, ContainerStopFailure, GlobalStopProgress,
    GlobalStopReport, Locale, Preferences, ProjectAction, ProjectSnapshot, RuntimeCapabilities,
    RuntimeError, RuntimeOperation, RuntimeSupervisor, ServiceAction, ServiceLogSnapshot,
    ServiceSnapshot, ServiceStatus, SnapshotPublisher, ThemePreference, VisibilityProbe,
};
use bollard::{
    Docker,
    errors::Error as DockerError,
    models::{ContainerStatsResponse, ContainerSummary, PortSummary},
    query_parameters::{EventsOptionsBuilder, ListContainersOptionsBuilder, StatsOptionsBuilder},
};
use futures_util::{
    StreamExt,
    future::{BoxFuture, join_all},
    stream,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ffi::OsString,
    path::PathBuf,
    sync::{Arc, RwLock as StandardRwLock},
    time::Duration,
};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock};

const COMPOSE_PROJECT_LABEL: &str = "com.docker.compose.project";
const COMPOSE_SERVICE_LABEL: &str = "com.docker.compose.service";
const COMPOSE_CONFIG_FILES_LABEL: &str = "com.docker.compose.project.config_files";
const COMPOSE_WORKING_DIRECTORY_LABEL: &str = "com.docker.compose.project.working_dir";
const EVENT_RETRY_LIMIT: Duration = Duration::from_secs(15);
const FULL_SYNCHRONIZATION_INTERVAL: Duration = Duration::from_secs(30);
const RESOURCE_SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const GLOBAL_STOP_CONCURRENCY: usize = 4;

pub struct DockerRuntimeSupervisor {
    action_gate: RwLock<()>,
    catalog: Mutex<ProjectCatalog>,
    client: Mutex<Option<Docker>>,
    operation_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    publisher: StandardRwLock<Option<SnapshotPublisher>>,
    reconciliation_lock: Mutex<()>,
    state: RwLock<DockerRuntimeState>,
}

struct DockerRuntimeState {
    snapshot: ApplicationSnapshot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ResourceSample {
    cpu_percent: Option<f64>,
    memory_bytes: Option<u64>,
}

impl DockerRuntimeSupervisor {
    pub fn new() -> Self {
        let catalog = ProjectCatalog::load_default();
        let preferences = catalog.preferences();
        let projects = catalog.decorate_projects(Vec::new());
        Self {
            action_gate: RwLock::new(()),
            catalog: Mutex::new(catalog),
            client: Mutex::new(None),
            operation_locks: Mutex::new(HashMap::new()),
            publisher: StandardRwLock::new(None),
            reconciliation_lock: Mutex::new(()),
            state: RwLock::new(DockerRuntimeState {
                snapshot: {
                    let mut snapshot = empty_snapshot();
                    snapshot.preferences = preferences;
                    snapshot.projects = projects;
                    snapshot
                },
            }),
        }
    }

    async fn docker(&self) -> Result<Docker, DockerError> {
        let mut client = self.client.lock().await;
        if let Some(docker) = client.as_ref() {
            return Ok(docker.clone());
        }

        let docker = Docker::connect_with_local_defaults()?;
        *client = Some(docker.clone());
        Ok(docker)
    }

    async fn reconcile(
        &self,
        include_resources: bool,
    ) -> Result<(ApplicationSnapshot, bool), RuntimeError> {
        let _reconciliation_guard = self.reconciliation_lock.lock().await;
        let previous = self.state.read().await.snapshot.clone();
        let result = self.read_engine(&previous, include_resources).await;

        match result {
            Ok(snapshot) => {
                let refresh_tray = tray_state_changed(&previous, &snapshot);
                self.state.write().await.snapshot = snapshot.clone();
                Ok((snapshot, refresh_tray))
            }
            Err(error) => {
                *self.client.lock().await = None;
                let runtime_error = runtime_error(&error);
                let mut state = self.state.write().await;
                state.snapshot.connection = connection_status(&error);
                state.snapshot.capabilities.lifecycle_actions = false;
                Err(runtime_error)
            }
        }
    }

    async fn read_engine(
        &self,
        previous: &ApplicationSnapshot,
        include_resources: bool,
    ) -> Result<ApplicationSnapshot, DockerError> {
        let docker = self.docker().await?;
        docker.ping().await?;

        let options = ListContainersOptionsBuilder::default().all(true).build();
        let containers = docker.list_containers(Some(options)).await?;
        let resources = if include_resources {
            Some(sample_resources(&docker, &containers).await)
        } else {
            None
        };

        let discoveries = discovered_projects(&containers);
        let mut snapshot = build_snapshot(containers, resources.as_ref(), previous);
        let mut catalog = self.catalog.lock().await;
        snapshot.projects = catalog.merge_projects(snapshot.projects, discoveries).await;
        snapshot.preferences = catalog.preferences();
        Ok(snapshot)
    }

    async fn publish_current(&self, publish: &SnapshotPublisher, refresh_tray: bool) {
        publish(self.state.read().await.snapshot.clone(), refresh_tray);
    }

    async fn publish_snapshot(&self, refresh_tray: bool) {
        let publisher = self
            .publisher
            .read()
            .expect("snapshot publisher lock should not be poisoned")
            .clone();
        if let Some(publisher) = publisher {
            publisher(self.state.read().await.snapshot.clone(), refresh_tray);
        }
    }

    async fn operation_lock(&self, key: &str) -> Arc<Mutex<()>> {
        let mut locks = self.operation_locks.lock().await;
        locks
            .entry(key.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn ensure_global_stop_is_idle(&self) -> Result<(), RuntimeError> {
        if self.state.read().await.snapshot.global_stop_in_progress {
            Err(RuntimeError::new("GLOBAL_STOP_IN_PROGRESS", true))
        } else {
            Ok(())
        }
    }

    async fn monitor_events(self: Arc<Self>, publish: SnapshotPublisher) {
        let mut retry_delay = Duration::from_secs(1);

        loop {
            let docker = match self.docker().await {
                Ok(docker) => docker,
                Err(error) => {
                    self.record_connection_error(&error).await;
                    self.publish_current(&publish, true).await;
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(EVENT_RETRY_LIMIT);
                    continue;
                }
            };

            match self.reconcile(false).await {
                Ok((snapshot, refresh_tray)) => {
                    retry_delay = Duration::from_secs(1);
                    publish(snapshot, refresh_tray);
                }
                Err(_) => {
                    self.publish_current(&publish, true).await;
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(EVENT_RETRY_LIMIT);
                    continue;
                }
            }

            let mut filters = HashMap::new();
            filters.insert("type", vec!["container"]);
            let options = EventsOptionsBuilder::default().filters(&filters).build();
            let mut events = Box::pin(docker.events(Some(options)));
            let mut full_synchronization = tokio::time::interval(FULL_SYNCHRONIZATION_INTERVAL);
            full_synchronization.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            full_synchronization.tick().await;

            loop {
                let update_result = tokio::select! {
                    _ = full_synchronization.tick() => self.reconcile(false).await,
                    event = events.next() => match event {
                        Some(Ok(_)) => self.reconcile(false).await,
                        Some(Err(error)) => {
                            self.record_connection_error(&error).await;
                            break;
                        }
                        None => break,
                    }
                };

                match update_result {
                    Ok((snapshot, refresh_tray)) => {
                        retry_delay = Duration::from_secs(1);
                        publish(snapshot, refresh_tray);
                    }
                    Err(_) => {
                        self.publish_current(&publish, true).await;
                        break;
                    }
                }
            }

            tokio::time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(EVENT_RETRY_LIMIT);
        }
    }

    async fn monitor_resources(
        self: Arc<Self>,
        publish: SnapshotPublisher,
        window_is_visible: VisibilityProbe,
    ) {
        let mut interval = tokio::time::interval(RESOURCE_SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            interval.tick().await;
            if !window_is_visible() || !self.is_connected().await {
                continue;
            }

            match self.reconcile(true).await {
                Ok((snapshot, refresh_tray)) => publish(snapshot, refresh_tray),
                Err(_) => self.publish_current(&publish, true).await,
            }
        }
    }

    async fn is_connected(&self) -> bool {
        self.state.read().await.snapshot.connection == ConnectionStatus::Connected
    }

    async fn record_connection_error(&self, error: &DockerError) {
        *self.client.lock().await = None;
        let mut state = self.state.write().await;
        state.snapshot.connection = connection_status(error);
        state.snapshot.capabilities.lifecycle_actions = false;
    }

    async fn execute_operation(
        &self,
        operation: RuntimeOperation,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        if let RuntimeOperation::ImportProject { paths } = operation {
            self.catalog.lock().await.import_project(paths).await?;
            {
                let _reconciliation_guard = self.reconciliation_lock.lock().await;
                let mut state = self.state.write().await;
                let catalog = self.catalog.lock().await;
                state.snapshot.projects =
                    catalog.decorate_projects(state.snapshot.projects.clone());
                state.snapshot.preferences = catalog.preferences();
            }
            return match self.reconcile(false).await {
                Ok((snapshot, _)) => Ok(snapshot),
                Err(_) => Ok(self.state.read().await.snapshot.clone()),
            };
        }

        match operation {
            RuntimeOperation::RunProjectAction { action, project_id } => {
                return self.run_project_action(&project_id, action).await;
            }
            RuntimeOperation::RunServiceAction { action, service_id } => {
                return self.run_service_action(&service_id, action).await;
            }
            RuntimeOperation::StopAll => return self.stop_all().await,
            _ => {}
        }

        let _reconciliation_guard = self.reconciliation_lock.lock().await;
        let mut state = self.state.write().await;
        let mut catalog = self.catalog.lock().await;

        match operation {
            RuntimeOperation::SetBulkSelected {
                project_id,
                selected,
                service_id,
            } => {
                let project = state
                    .snapshot
                    .projects
                    .iter_mut()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| RuntimeError::new("PROJECT_NOT_FOUND", false))?;
                let service = project
                    .services
                    .iter_mut()
                    .find(|service| service.id == service_id)
                    .ok_or_else(|| RuntimeError::new("SERVICE_NOT_FOUND", false))?;
                catalog.set_bulk_selected(&project.id, &service.name, selected)?;
                service.bulk_selected = selected;
            }
            RuntimeOperation::SetLanguage(language) => {
                catalog.set_language(language)?;
                state.snapshot.preferences.language = language;
            }
            RuntimeOperation::RemoveProject { project_id } => {
                if state
                    .snapshot
                    .projects
                    .iter()
                    .all(|project| project.id != project_id)
                {
                    return Err(RuntimeError::new("PROJECT_NOT_FOUND", false));
                }
                catalog.remove_project(&project_id)?;
                state.snapshot.projects =
                    catalog.decorate_projects(state.snapshot.projects.clone());
            }
            RuntimeOperation::SetProjectActive { active, project_id } => {
                let project = state
                    .snapshot
                    .projects
                    .iter_mut()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| RuntimeError::new("PROJECT_NOT_FOUND", false))?;
                catalog.set_project_active(&project.id, active)?;
                project.active = active;
            }
            RuntimeOperation::SetProjectProfiles {
                profiles,
                project_id,
            } => {
                let project = state
                    .snapshot
                    .projects
                    .iter_mut()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| RuntimeError::new("PROJECT_NOT_FOUND", false))?;
                catalog.set_project_profiles(&project.id, profiles)?;
                catalog.refresh_project(project);
            }
            RuntimeOperation::SetTheme(theme) => {
                catalog.set_theme(theme)?;
                state.snapshot.preferences.theme = theme;
            }
            RuntimeOperation::RunProjectAction { .. }
            | RuntimeOperation::RunServiceAction { .. }
            | RuntimeOperation::StopAll => unreachable!(),
            RuntimeOperation::ImportProject { .. } => unreachable!(),
        }

        Ok(state.snapshot.clone())
    }

    async fn run_project_action(
        &self,
        project_id: &str,
        action: ProjectAction,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        self.ensure_global_stop_is_idle().await?;
        let _action_guard = self.action_gate.read().await;
        self.ensure_global_stop_is_idle().await?;
        let project_lock = self.operation_lock(project_id).await;
        let _project_guard = project_lock
            .try_lock()
            .map_err(|_| RuntimeError::new("PROJECT_OPERATION_IN_PROGRESS", true))?;

        let (context, services) = {
            let state = self.state.read().await;
            let project = state
                .snapshot
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .ok_or_else(|| RuntimeError::new("PROJECT_NOT_FOUND", false))?;
            let services = project
                .services
                .iter()
                .filter(|service| service.included && service.bulk_selected)
                .map(|service| service.name.clone())
                .collect::<Vec<_>>();
            let context = self.catalog.lock().await.execution_context(project_id)?;
            (context, services)
        };
        if services.is_empty() {
            return Err(RuntimeError::new("NO_SERVICES_SELECTED", false));
        }

        let service_action = match action {
            ProjectAction::StartSelected => ServiceAction::Start,
            ProjectAction::StopSelected => ServiceAction::Stop,
        };
        run_compose(&context, service_action, &services).await?;
        self.reconcile(false).await.map(|(snapshot, _)| snapshot)
    }

    async fn run_service_action(
        &self,
        service_id: &str,
        action: ServiceAction,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        self.ensure_global_stop_is_idle().await?;
        let _action_guard = self.action_gate.read().await;
        self.ensure_global_stop_is_idle().await?;

        let compose_service = {
            let state = self.state.read().await;
            state.snapshot.projects.iter().find_map(|project| {
                project
                    .services
                    .iter()
                    .find(|service| service.id == service_id)
                    .map(|service| (project.id.clone(), service.name.clone(), service.included))
            })
        };

        if let Some((project_id, service_name, included)) = compose_service {
            if !included {
                return Err(RuntimeError::new("SERVICE_NOT_INCLUDED", false));
            }
            let project_lock = self.operation_lock(&project_id).await;
            let _project_guard = project_lock
                .try_lock()
                .map_err(|_| RuntimeError::new("PROJECT_OPERATION_IN_PROGRESS", true))?;
            let context = self.catalog.lock().await.execution_context(&project_id)?;
            run_compose(&context, action, &[service_name]).await?;
        } else {
            let container_id = service_id
                .strip_prefix("container:")
                .ok_or_else(|| RuntimeError::new("SERVICE_NOT_FOUND", false))?;
            let container_exists = self
                .state
                .read()
                .await
                .snapshot
                .standalone_containers
                .iter()
                .any(|service| service.id == service_id);
            if !container_exists {
                return Err(RuntimeError::new("SERVICE_NOT_FOUND", false));
            }
            let container_lock = self.operation_lock(service_id).await;
            let _container_guard = container_lock
                .try_lock()
                .map_err(|_| RuntimeError::new("CONTAINER_OPERATION_IN_PROGRESS", true))?;
            run_container_action(
                &self.docker().await.map_err(|error| runtime_error(&error))?,
                container_id,
                action,
            )
            .await?;
        }

        self.reconcile(false).await.map(|(snapshot, _)| snapshot)
    }

    async fn stop_all(&self) -> Result<ApplicationSnapshot, RuntimeError> {
        let report_sequence = {
            let mut state = self.state.write().await;
            if state.snapshot.global_stop_in_progress {
                return Err(RuntimeError::new("GLOBAL_STOP_IN_PROGRESS", true));
            }
            let report_sequence = state
                .snapshot
                .global_stop_report
                .as_ref()
                .map_or(1, |report| report.sequence + 1);
            state.snapshot.global_stop_in_progress = true;
            state.snapshot.global_stop_progress = Some(GlobalStopProgress {
                completed: 0,
                total: 0,
            });
            state.snapshot.global_stop_report = None;
            report_sequence
        };
        self.publish_snapshot(true).await;

        let _action_guard = self.action_gate.write().await;
        let docker = match self.docker().await {
            Ok(docker) => docker,
            Err(error) => {
                self.record_connection_error(&error).await;
                self.finish_failed_global_stop().await;
                return Err(runtime_error(&error));
            }
        };
        let options = ListContainersOptionsBuilder::default().all(true).build();
        let containers = match docker.list_containers(Some(options)).await {
            Ok(containers) => containers,
            Err(error) => {
                self.record_connection_error(&error).await;
                self.finish_failed_global_stop().await;
                return Err(runtime_error(&error));
            }
        };
        let targets = global_stop_targets(&containers);
        {
            let mut state = self.state.write().await;
            state.snapshot.global_stop_progress = Some(GlobalStopProgress {
                completed: 0,
                total: targets.len(),
            });
        }
        self.publish_snapshot(true).await;

        let total = targets.len();
        let mut results = Vec::with_capacity(total);
        let mut stops = stream::iter(targets.into_iter().map(|target| {
            let docker = docker.clone();
            async move {
                let result = stop_container(&docker, &target).await;
                (target, result)
            }
        }))
        .buffer_unordered(GLOBAL_STOP_CONCURRENCY);
        while let Some(result) = stops.next().await {
            results.push(result);
            {
                let mut state = self.state.write().await;
                state.snapshot.global_stop_progress = Some(GlobalStopProgress {
                    completed: results.len(),
                    total,
                });
            }
            self.publish_snapshot(false).await;
        }

        let report = global_stop_report(results, report_sequence);
        {
            let mut state = self.state.write().await;
            state.snapshot.global_stop_in_progress = false;
            state.snapshot.global_stop_progress = None;
            state.snapshot.global_stop_report = Some(report);
        }
        self.publish_snapshot(true).await;

        match self.reconcile(false).await {
            Ok((snapshot, _)) => Ok(snapshot),
            Err(_) => Ok(self.state.read().await.snapshot.clone()),
        }
    }

    async fn finish_failed_global_stop(&self) {
        let mut state = self.state.write().await;
        state.snapshot.global_stop_in_progress = false;
        state.snapshot.global_stop_progress = None;
        drop(state);
        self.publish_snapshot(true).await;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalStopTarget {
    container_id: String,
    container_name: String,
    paused: bool,
}

fn compose_arguments(
    context: &ComposeExecutionContext,
    action: ServiceAction,
    services: &[String],
) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("compose")];
    for compose_file in &context.compose_files {
        arguments.push(OsString::from("-f"));
        arguments.push(compose_file.as_os_str().to_owned());
    }
    for profile in &context.enabled_profiles {
        arguments.push(OsString::from("--profile"));
        arguments.push(OsString::from(profile));
    }
    match action {
        ServiceAction::Start => {
            arguments.extend([OsString::from("up"), OsString::from("-d")]);
        }
        ServiceAction::Stop => arguments.push(OsString::from("stop")),
        ServiceAction::Restart => arguments.push(OsString::from("restart")),
        ServiceAction::Resume => arguments.push(OsString::from("unpause")),
    }
    arguments.extend(services.iter().map(OsString::from));
    arguments
}

async fn run_compose(
    context: &ComposeExecutionContext,
    action: ServiceAction,
    services: &[String],
) -> Result<(), RuntimeError> {
    let output = Command::new("docker")
        .args(compose_arguments(context, action, services))
        .current_dir(&context.working_directory)
        .output()
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RuntimeError::new("COMPOSE_CLI_NOT_FOUND", false)
            } else {
                RuntimeError::new("COMPOSE_ACTION_FAILED", true)
            }
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(RuntimeError::new("COMPOSE_ACTION_FAILED", true))
    }
}

async fn run_container_action(
    docker: &Docker,
    container_id: &str,
    action: ServiceAction,
) -> Result<(), RuntimeError> {
    let result = match action {
        ServiceAction::Start => docker.start_container(container_id, None).await,
        ServiceAction::Stop => docker.stop_container(container_id, None).await,
        ServiceAction::Restart => docker.restart_container(container_id, None).await,
        ServiceAction::Resume => docker.unpause_container(container_id).await,
    };
    result.map_err(|_| RuntimeError::new("CONTAINER_ACTION_FAILED", true))
}

fn global_stop_targets(containers: &[ContainerSummary]) -> Vec<GlobalStopTarget> {
    containers
        .iter()
        .filter_map(|container| {
            let state = container.state.as_ref()?.as_ref();
            if !matches!(state, "running" | "paused" | "restarting") {
                return None;
            }
            Some(GlobalStopTarget {
                container_id: container.id.clone()?,
                container_name: container_name(container),
                paused: state == "paused",
            })
        })
        .collect()
}

async fn stop_container(
    docker: &Docker,
    target: &GlobalStopTarget,
) -> Result<(), ContainerStopFailure> {
    if target.paused
        && docker
            .unpause_container(&target.container_id)
            .await
            .is_err()
    {
        return Err(ContainerStopFailure {
            code: "CONTAINER_UNPAUSE_FAILED".to_owned(),
            container_name: target.container_name.clone(),
            retryable: true,
        });
    }
    docker
        .stop_container(&target.container_id, None)
        .await
        .map_err(|_| ContainerStopFailure {
            code: "CONTAINER_STOP_FAILED".to_owned(),
            container_name: target.container_name.clone(),
            retryable: true,
        })
}

fn global_stop_report(
    results: Vec<(GlobalStopTarget, Result<(), ContainerStopFailure>)>,
    sequence: u64,
) -> GlobalStopReport {
    let total = results.len();
    let mut failures = results
        .into_iter()
        .filter_map(|(_, result)| result.err())
        .collect::<Vec<_>>();
    failures.sort_by(|left, right| left.container_name.cmp(&right.container_name));
    GlobalStopReport {
        stopped: total - failures.len(),
        failures,
        sequence,
        total,
    }
}

impl RuntimeSupervisor for DockerRuntimeSupervisor {
    fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move { self.execute_operation(operation).await })
    }

    fn logs<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> BoxFuture<'a, Result<ServiceLogSnapshot, RuntimeError>> {
        Box::pin(async move { Err(RuntimeError::new("LOGS_NOT_AVAILABLE", false)) })
    }

    fn snapshot(&self) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move { Ok(self.state.read().await.snapshot.clone()) })
    }

    fn start_monitoring(
        self: Arc<Self>,
        publish: SnapshotPublisher,
        window_is_visible: VisibilityProbe,
    ) {
        *self
            .publisher
            .write()
            .expect("snapshot publisher lock should not be poisoned") = Some(publish.clone());
        tauri::async_runtime::spawn(self.clone().monitor_events(publish.clone()));
        tauri::async_runtime::spawn(self.monitor_resources(publish, window_is_visible));
    }

    fn synchronize(
        &self,
        include_resources: bool,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move {
            self.reconcile(include_resources)
                .await
                .map(|(snapshot, _)| snapshot)
        })
    }
}

async fn sample_resources(
    docker: &Docker,
    containers: &[ContainerSummary],
) -> HashMap<String, ResourceSample> {
    let samples = containers
        .iter()
        .filter(|container| {
            container
                .state
                .as_ref()
                .is_some_and(|state| matches!(state.as_ref(), "running" | "paused"))
        })
        .filter_map(|container| container.id.clone())
        .map(|container_id| {
            let docker = docker.clone();
            async move {
                let options = StatsOptionsBuilder::default()
                    .stream(false)
                    .one_shot(false)
                    .build();
                let mut stream = Box::pin(docker.stats(&container_id, Some(options)));
                let sample = match stream.next().await {
                    Some(Ok(stats)) => resource_sample(&stats),
                    _ => ResourceSample::default(),
                };
                (container_id, sample)
            }
        });

    join_all(samples).await.into_iter().collect()
}

fn build_snapshot(
    containers: Vec<ContainerSummary>,
    resources: Option<&HashMap<String, ResourceSample>>,
    previous: &ApplicationSnapshot,
) -> ApplicationSnapshot {
    let previous_services: HashMap<_, _> = previous
        .projects
        .iter()
        .flat_map(|project| project.services.iter())
        .chain(previous.standalone_containers.iter())
        .map(|service| (service.id.as_str(), service))
        .collect();
    let previous_projects: HashMap<_, _> = previous
        .projects
        .iter()
        .map(|project| (project.id.as_str(), project))
        .collect();

    let mut compose_projects: BTreeMap<String, BTreeMap<String, Vec<&ContainerSummary>>> =
        BTreeMap::new();
    let mut standalone = Vec::new();

    for container in &containers {
        let labels = container.labels.as_ref();
        let project = labels.and_then(|labels| labels.get(COMPOSE_PROJECT_LABEL));
        let service = labels.and_then(|labels| labels.get(COMPOSE_SERVICE_LABEL));

        if let (Some(project), Some(service)) = (project, service) {
            compose_projects
                .entry(project.clone())
                .or_default()
                .entry(service.clone())
                .or_default()
                .push(container);
        } else {
            standalone.push(container);
        }
    }

    let projects = compose_projects
        .into_iter()
        .map(|(project_name, services)| {
            let project_id = compose_project_id(&project_name);
            let active = previous_projects
                .get(project_id.as_str())
                .is_some_and(|project| project.active);
            let services = services
                .into_iter()
                .map(|(service_name, containers)| {
                    let service_id = compose_service_id(&project_name, &service_name);
                    let previous_service = previous_services.get(service_id.as_str()).copied();
                    service_snapshot(
                        service_id,
                        service_name,
                        containers,
                        resources,
                        previous_service,
                        true,
                    )
                })
                .collect();

            ProjectSnapshot {
                active,
                id: project_id,
                name: project_name,
                profiles: Vec::new(),
                services,
            }
        })
        .collect();

    let standalone_containers = standalone
        .into_iter()
        .filter_map(|container| {
            let container_id = container.id.as_deref()?;
            let service_id = format!("container:{container_id}");
            Some(service_snapshot(
                service_id.clone(),
                container_name(container),
                vec![container],
                resources,
                previous_services.get(service_id.as_str()).copied(),
                false,
            ))
        })
        .collect();

    ApplicationSnapshot {
        activity: previous.activity.clone(),
        capabilities: RuntimeCapabilities {
            lifecycle_actions: true,
            logs: false,
        },
        connection: ConnectionStatus::Connected,
        global_stop_in_progress: previous.global_stop_in_progress,
        global_stop_progress: previous.global_stop_progress.clone(),
        global_stop_report: previous.global_stop_report.clone(),
        preferences: previous.preferences.clone(),
        projects,
        standalone_containers,
    }
}

fn discovered_projects(containers: &[ContainerSummary]) -> Vec<DiscoveredProject> {
    let mut discoveries: BTreeMap<String, DiscoveredProject> = BTreeMap::new();

    for container in containers {
        let Some(labels) = container.labels.as_ref() else {
            continue;
        };
        let (Some(project_name), Some(service_name), Some(working_directory), Some(config_files)) = (
            labels.get(COMPOSE_PROJECT_LABEL),
            labels.get(COMPOSE_SERVICE_LABEL),
            labels.get(COMPOSE_WORKING_DIRECTORY_LABEL),
            labels.get(COMPOSE_CONFIG_FILES_LABEL),
        ) else {
            continue;
        };
        let working_directory = PathBuf::from(working_directory);
        let compose_files: Vec<_> = config_files
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    working_directory.join(path)
                }
            })
            .collect();
        if compose_files.is_empty() {
            continue;
        }

        let project_id = compose_project_id(project_name);
        let discovery =
            discoveries
                .entry(project_id.clone())
                .or_insert_with(|| DiscoveredProject {
                    compose_files,
                    id: project_id,
                    name: project_name.clone(),
                    running_services: BTreeSet::new(),
                    working_directory,
                });
        if container_state(container).can_stop() {
            discovery.running_services.insert(service_name.clone());
        }
    }

    discoveries.into_values().collect()
}

fn service_snapshot(
    id: String,
    name: String,
    containers: Vec<&ContainerSummary>,
    resources: Option<&HashMap<String, ResourceSample>>,
    previous: Option<&ServiceSnapshot>,
    default_bulk_selected: bool,
) -> ServiceSnapshot {
    let statuses: Vec<_> = containers
        .iter()
        .map(|container| container_state(container))
        .collect();
    let status = aggregate_status(&statuses);
    let ports = aggregate_ports(&containers);
    let aggregated_resources =
        resources.map(|resources| aggregate_resources(&containers, resources));
    let (cpu_percent, memory_bytes) = if status.can_stop() {
        aggregated_resources
            .flatten()
            .map(|sample| (sample.cpu_percent, sample.memory_bytes))
            .or_else(|| previous.map(|service| (service.cpu_percent, service.memory_bytes)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    ServiceSnapshot {
        bulk_selected: previous
            .map(|service| service.bulk_selected)
            .unwrap_or(default_bulk_selected),
        cpu_percent,
        id,
        included: true,
        memory_bytes,
        name,
        ports,
        profiles: Vec::new(),
        status,
    }
}

fn container_state(container: &ContainerSummary) -> ServiceStatus {
    match container.state.as_ref().map(AsRef::as_ref) {
        Some("running") => match container
            .health
            .as_ref()
            .and_then(|health| health.status.as_ref())
            .map(AsRef::as_ref)
        {
            Some("healthy") => ServiceStatus::Healthy,
            Some("unhealthy") => ServiceStatus::Unhealthy,
            Some("starting") => ServiceStatus::Starting,
            _ if container
                .status
                .as_deref()
                .is_some_and(|status| status.contains("(healthy)")) =>
            {
                ServiceStatus::Healthy
            }
            _ if container
                .status
                .as_deref()
                .is_some_and(|status| status.contains("(unhealthy)")) =>
            {
                ServiceStatus::Unhealthy
            }
            _ => ServiceStatus::Running,
        },
        Some("paused") => ServiceStatus::Paused,
        Some("restarting") | Some("stopping") => ServiceStatus::Starting,
        Some("dead") | Some("removing") => ServiceStatus::Error,
        Some("exited") if exited_unsuccessfully(container.status.as_deref()) => {
            ServiceStatus::Error
        }
        Some("created") | Some("exited") => ServiceStatus::Stopped,
        _ => ServiceStatus::Error,
    }
}

fn exited_unsuccessfully(status: Option<&str>) -> bool {
    status.is_some_and(|status| status.starts_with("Exited (") && !status.starts_with("Exited (0)"))
}

fn aggregate_status(statuses: &[ServiceStatus]) -> ServiceStatus {
    if statuses.is_empty() {
        return ServiceStatus::NotCreated;
    }

    for status in [
        ServiceStatus::Error,
        ServiceStatus::Unhealthy,
        ServiceStatus::Starting,
        ServiceStatus::Paused,
    ] {
        if statuses.contains(&status) {
            return status;
        }
    }

    if statuses
        .iter()
        .all(|status| *status == ServiceStatus::Stopped)
    {
        ServiceStatus::Stopped
    } else if statuses
        .iter()
        .all(|status| *status == ServiceStatus::Healthy)
    {
        ServiceStatus::Healthy
    } else {
        ServiceStatus::Running
    }
}

fn aggregate_ports(containers: &[&ContainerSummary]) -> Vec<String> {
    let mut ports: Vec<_> = containers
        .iter()
        .flat_map(|container| container.ports.iter().flatten())
        .map(format_port)
        .collect();
    ports.sort();
    ports.dedup();
    ports
}

fn format_port(port: &PortSummary) -> String {
    let protocol = port
        .typ
        .as_ref()
        .map(AsRef::as_ref)
        .filter(|protocol| *protocol != "tcp" && !protocol.is_empty())
        .map(|protocol| format!("/{protocol}"))
        .unwrap_or_default();

    match port.public_port {
        Some(public_port) => {
            let host = port
                .ip
                .as_deref()
                .filter(|ip| !ip.is_empty() && *ip != "0.0.0.0" && *ip != "::")
                .map(|ip| format!("{ip}:"))
                .unwrap_or_default();
            format!("{host}{public_port}:{}{protocol}", port.private_port)
        }
        None => format!("{}{protocol}", port.private_port),
    }
}

fn aggregate_resources(
    containers: &[&ContainerSummary],
    resources: &HashMap<String, ResourceSample>,
) -> Option<ResourceSample> {
    let samples: Vec<_> = containers
        .iter()
        .filter_map(|container| container.id.as_ref())
        .filter_map(|container_id| resources.get(container_id))
        .collect();
    if samples.is_empty() {
        return None;
    }

    let cpu_values: Vec<_> = samples
        .iter()
        .filter_map(|sample| sample.cpu_percent)
        .collect();
    let memory_values: Vec<_> = samples
        .iter()
        .filter_map(|sample| sample.memory_bytes)
        .collect();
    if cpu_values.is_empty() && memory_values.is_empty() {
        return None;
    }

    Some(ResourceSample {
        cpu_percent: (!cpu_values.is_empty()).then(|| cpu_values.iter().sum()),
        memory_bytes: (!memory_values.is_empty()).then(|| memory_values.iter().sum()),
    })
}

fn resource_sample(stats: &ContainerStatsResponse) -> ResourceSample {
    let cpu_percent = stats
        .cpu_stats
        .as_ref()
        .zip(stats.precpu_stats.as_ref())
        .and_then(|(current, previous)| {
            let current_total = current.cpu_usage.as_ref()?.total_usage?;
            let previous_total = previous.cpu_usage.as_ref()?.total_usage?;
            let current_system = current.system_cpu_usage?;
            let previous_system = previous.system_cpu_usage?;
            let cpu_delta = current_total.saturating_sub(previous_total) as f64;
            let system_delta = current_system.saturating_sub(previous_system) as f64;
            if cpu_delta == 0.0 || system_delta == 0.0 {
                return Some(0.0);
            }
            let online_cpus = current.online_cpus.map(f64::from).or_else(|| {
                current
                    .cpu_usage
                    .as_ref()?
                    .percpu_usage
                    .as_ref()
                    .map(|usage| usage.len() as f64)
            })?;
            Some((cpu_delta / system_delta) * online_cpus * 100.0)
        });

    let memory_bytes = stats.memory_stats.as_ref().and_then(|memory| {
        let usage = memory.usage?;
        let inactive = memory.stats.as_ref().and_then(|values| {
            values
                .get("total_inactive_file")
                .or_else(|| values.get("inactive_file"))
                .copied()
        });
        Some(usage.saturating_sub(inactive.unwrap_or(0)))
    });

    ResourceSample {
        cpu_percent,
        memory_bytes,
    }
}

fn container_name(container: &ContainerSummary) -> String {
    container
        .names
        .as_ref()
        .and_then(|names| names.first())
        .map(|name| name.trim_start_matches('/').to_owned())
        .or_else(|| container.id.as_ref().map(|id| short_id(id).to_owned()))
        .unwrap_or_else(|| "unknown-container".to_owned())
}

fn short_id(id: &str) -> &str {
    &id[..id.len().min(12)]
}

fn compose_project_id(project: &str) -> String {
    format!("compose:{project}")
}

fn compose_service_id(project: &str, service: &str) -> String {
    format!("compose:{project}:{service}")
}

fn connection_status(error: &DockerError) -> ConnectionStatus {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("permission denied") || message.contains("status code 403") {
        ConnectionStatus::PermissionDenied
    } else {
        ConnectionStatus::Disconnected
    }
}

fn runtime_error(error: &DockerError) -> RuntimeError {
    match connection_status(error) {
        ConnectionStatus::PermissionDenied => RuntimeError::new("DOCKER_PERMISSION_DENIED", false),
        _ => RuntimeError::new("DOCKER_UNAVAILABLE", true),
    }
}

fn tray_state_changed(previous: &ApplicationSnapshot, next: &ApplicationSnapshot) -> bool {
    if previous.connection != next.connection
        || previous.capabilities != next.capabilities
        || previous.projects.len() != next.projects.len()
    {
        return true;
    }

    previous
        .projects
        .iter()
        .zip(&next.projects)
        .any(|(left, right)| {
            left.id != right.id
                || left.active != right.active
                || left.profiles != right.profiles
                || left.services.len() != right.services.len()
                || left
                    .services
                    .iter()
                    .zip(&right.services)
                    .any(|(left, right)| {
                        left.id != right.id
                            || left.included != right.included
                            || left.status != right.status
                    })
        })
}

fn empty_snapshot() -> ApplicationSnapshot {
    ApplicationSnapshot {
        activity: Vec::new(),
        capabilities: RuntimeCapabilities {
            lifecycle_actions: false,
            logs: false,
        },
        connection: ConnectionStatus::Connecting,
        global_stop_in_progress: false,
        global_stop_progress: None,
        global_stop_report: None,
        preferences: Preferences {
            language: Locale::system(),
            theme: ThemePreference::System,
        },
        projects: Vec::new(),
        standalone_containers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::{
        ContainerCpuStats, ContainerCpuUsage, ContainerMemoryStats, ContainerSummaryStateEnum,
    };

    fn container(
        id: &str,
        name: &str,
        state: &str,
        project: Option<(&str, &str)>,
        ports: Vec<PortSummary>,
    ) -> ContainerSummary {
        let labels = project.map(|(project, service)| {
            HashMap::from([
                (COMPOSE_PROJECT_LABEL.to_owned(), project.to_owned()),
                (COMPOSE_SERVICE_LABEL.to_owned(), service.to_owned()),
            ])
        });

        ContainerSummary {
            id: Some(id.to_owned()),
            labels,
            names: Some(vec![format!("/{name}")]),
            ports: Some(ports),
            state: Some(
                state
                    .parse::<ContainerSummaryStateEnum>()
                    .expect("fixture state should be valid"),
            ),
            status: Some(if state == "exited" {
                "Exited (0)".to_owned()
            } else {
                "Up 10 seconds".to_owned()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn groups_compose_replicas_and_keeps_standalone_containers_separate() {
        let containers = vec![
            container(
                "web-1",
                "shop-web-1",
                "running",
                Some(("shop", "web")),
                vec![PortSummary {
                    private_port: 3000,
                    public_port: Some(8080),
                    ..Default::default()
                }],
            ),
            container(
                "web-2",
                "shop-web-2",
                "running",
                Some(("shop", "web")),
                Vec::new(),
            ),
            container("redis", "shared-redis", "exited", None, Vec::new()),
        ];
        let resources = HashMap::from([
            (
                "web-1".to_owned(),
                ResourceSample {
                    cpu_percent: Some(1.25),
                    memory_bytes: Some(100),
                },
            ),
            (
                "web-2".to_owned(),
                ResourceSample {
                    cpu_percent: Some(0.75),
                    memory_bytes: Some(200),
                },
            ),
        ]);

        let snapshot = build_snapshot(containers, Some(&resources), &empty_snapshot());

        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.projects[0].id, "compose:shop");
        assert_eq!(snapshot.projects[0].services.len(), 1);
        assert_eq!(snapshot.projects[0].services[0].cpu_percent, Some(2.0));
        assert_eq!(snapshot.projects[0].services[0].memory_bytes, Some(300));
        assert_eq!(snapshot.projects[0].services[0].ports, vec!["8080:3000"]);
        assert_eq!(snapshot.standalone_containers.len(), 1);
        assert_eq!(snapshot.standalone_containers[0].name, "shared-redis");
    }

    #[test]
    fn maps_and_aggregates_runtime_states() {
        assert_eq!(
            aggregate_status(&[ServiceStatus::Healthy, ServiceStatus::Running]),
            ServiceStatus::Running
        );
        assert_eq!(
            aggregate_status(&[ServiceStatus::Healthy, ServiceStatus::Unhealthy]),
            ServiceStatus::Unhealthy
        );
        assert_eq!(
            container_state(&container("one", "one", "paused", None, Vec::new())),
            ServiceStatus::Paused
        );
    }

    #[test]
    fn reads_compose_source_metadata_from_container_labels() {
        let mut labeled_container = container(
            "web-1",
            "zerniki-web-1",
            "running",
            Some(("zerniki", "web")),
            Vec::new(),
        );
        let labels = labeled_container
            .labels
            .as_mut()
            .expect("compose fixture should have labels");
        labels.insert(
            COMPOSE_WORKING_DIRECTORY_LABEL.to_owned(),
            "/workspace/zerniki".to_owned(),
        );
        labels.insert(
            COMPOSE_CONFIG_FILES_LABEL.to_owned(),
            "compose.yaml,/workspace/shared.yaml".to_owned(),
        );

        let projects = discovered_projects(&[labeled_container]);

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "compose:zerniki");
        assert_eq!(
            projects[0].compose_files,
            vec![
                PathBuf::from("/workspace/zerniki/compose.yaml"),
                PathBuf::from("/workspace/shared.yaml")
            ]
        );
        assert!(projects[0].running_services.contains("web"));
    }

    #[test]
    fn distinguishes_socket_permission_errors_from_disconnections() {
        let permission_error = DockerError::IOError {
            err: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };
        let disconnected_error = DockerError::IOError {
            err: std::io::Error::from(std::io::ErrorKind::ConnectionRefused),
        };

        assert_eq!(
            connection_status(&permission_error),
            ConnectionStatus::PermissionDenied
        );
        assert_eq!(
            connection_status(&disconnected_error),
            ConnectionStatus::Disconnected
        );
    }

    #[test]
    fn calculates_cpu_and_memory_like_docker_stats() {
        let stats = ContainerStatsResponse {
            cpu_stats: Some(ContainerCpuStats {
                cpu_usage: Some(ContainerCpuUsage {
                    total_usage: Some(300),
                    ..Default::default()
                }),
                online_cpus: Some(4),
                system_cpu_usage: Some(2_000),
                ..Default::default()
            }),
            precpu_stats: Some(ContainerCpuStats {
                cpu_usage: Some(ContainerCpuUsage {
                    total_usage: Some(200),
                    ..Default::default()
                }),
                system_cpu_usage: Some(1_000),
                ..Default::default()
            }),
            memory_stats: Some(ContainerMemoryStats {
                stats: Some(HashMap::from([("inactive_file".to_owned(), 200)])),
                usage: Some(1_000),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            resource_sample(&stats),
            ResourceSample {
                cpu_percent: Some(40.0),
                memory_bytes: Some(800),
            }
        );
    }

    #[test]
    fn keeps_last_resource_sample_when_docker_stats_is_temporarily_empty() {
        let containers = vec![container(
            "web-1",
            "shop-web-1",
            "running",
            Some(("shop", "web")),
            Vec::new(),
        )];
        let initial_resources = HashMap::from([(
            "web-1".to_owned(),
            ResourceSample {
                cpu_percent: Some(1.5),
                memory_bytes: Some(256),
            },
        )]);
        let previous = build_snapshot(
            containers.clone(),
            Some(&initial_resources),
            &empty_snapshot(),
        );
        let unavailable_resources =
            HashMap::from([("web-1".to_owned(), ResourceSample::default())]);

        let next = build_snapshot(containers, Some(&unavailable_resources), &previous);

        assert_eq!(next.projects[0].services[0].cpu_percent, Some(1.5));
        assert_eq!(next.projects[0].services[0].memory_bytes, Some(256));
    }

    #[test]
    fn preserves_project_and_bulk_selection_during_reconciliation() {
        let containers = vec![container(
            "web-1",
            "shop-web-1",
            "running",
            Some(("shop", "web")),
            Vec::new(),
        )];
        let mut previous = build_snapshot(containers.clone(), None, &empty_snapshot());
        previous.projects[0].active = true;
        previous.projects[0].services[0].bulk_selected = false;
        previous.global_stop_report = Some(GlobalStopReport {
            failures: Vec::new(),
            sequence: 3,
            stopped: 1,
            total: 1,
        });

        let next = build_snapshot(containers, None, &previous);

        assert!(next.projects[0].active);
        assert!(!next.projects[0].services[0].bulk_selected);
        assert_eq!(next.global_stop_report, previous.global_stop_report);
    }

    #[test]
    fn builds_compose_arguments_without_a_shell_and_preserves_file_order() {
        let context = ComposeExecutionContext {
            compose_files: vec![
                PathBuf::from("/workspace/compose.yaml"),
                PathBuf::from("/workspace/compose.local.yaml"),
            ],
            enabled_profiles: vec!["test".to_owned()],
            working_directory: PathBuf::from("/workspace"),
        };

        assert_eq!(
            compose_arguments(&context, ServiceAction::Start, &["api".to_owned()]),
            vec![
                "compose",
                "-f",
                "/workspace/compose.yaml",
                "-f",
                "/workspace/compose.local.yaml",
                "--profile",
                "test",
                "up",
                "-d",
                "api",
            ]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
        );
        let action_tail = |action| {
            compose_arguments(&context, action, &["api".to_owned()])
                .into_iter()
                .skip(7)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            action_tail(ServiceAction::Stop),
            ["stop", "api"]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            action_tail(ServiceAction::Restart),
            ["restart", "api"]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            action_tail(ServiceAction::Resume),
            ["unpause", "api"]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn global_stop_targets_every_running_or_paused_container() {
        let containers = vec![
            container(
                "one",
                "known-project-one",
                "running",
                Some(("known-one", "api")),
                Vec::new(),
            ),
            container(
                "two",
                "known-project-two",
                "running",
                Some(("known-two", "worker")),
                Vec::new(),
            ),
            container(
                "three",
                "unknown-project",
                "running",
                Some(("unknown", "web")),
                Vec::new(),
            ),
            container("four", "standalone", "paused", None, Vec::new()),
            container("five", "already-stopped", "exited", None, Vec::new()),
        ];

        let targets = global_stop_targets(&containers);

        assert_eq!(targets.len(), 4);
        assert!(
            targets
                .iter()
                .any(|target| target.container_name == "known-project-one")
        );
        assert!(
            targets
                .iter()
                .any(|target| target.container_name == "known-project-two")
        );
        assert!(
            targets
                .iter()
                .any(|target| target.container_name == "unknown-project")
        );
        assert!(
            targets
                .iter()
                .any(|target| target.container_name == "standalone" && target.paused)
        );
    }

    #[test]
    fn reports_full_and_partial_global_stop_results() {
        let first = GlobalStopTarget {
            container_id: "one".to_owned(),
            container_name: "alpha".to_owned(),
            paused: false,
        };
        let second = GlobalStopTarget {
            container_id: "two".to_owned(),
            container_name: "beta".to_owned(),
            paused: false,
        };
        let full = global_stop_report(vec![(first.clone(), Ok(())), (second.clone(), Ok(()))], 1);
        let partial = global_stop_report(
            vec![
                (first, Ok(())),
                (
                    second,
                    Err(ContainerStopFailure {
                        code: "CONTAINER_STOP_FAILED".to_owned(),
                        container_name: "beta".to_owned(),
                        retryable: true,
                    }),
                ),
            ],
            2,
        );

        assert_eq!((full.stopped, full.total, full.failures.len()), (2, 2, 0));
        assert_eq!(
            (partial.stopped, partial.total, partial.failures.len()),
            (1, 2, 1)
        );
        assert_eq!(partial.failures[0].container_name, "beta");
        assert_eq!(partial.sequence, 2);
    }
}
