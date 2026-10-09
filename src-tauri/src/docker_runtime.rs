use crate::runtime::{
    ApplicationSnapshot, ConnectionStatus, Locale, Preferences, ProjectSnapshot,
    RuntimeCapabilities, RuntimeError, RuntimeOperation, RuntimeSupervisor, ServiceLogSnapshot,
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
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, RwLock};

const COMPOSE_PROJECT_LABEL: &str = "com.docker.compose.project";
const COMPOSE_SERVICE_LABEL: &str = "com.docker.compose.service";
const EVENT_RETRY_LIMIT: Duration = Duration::from_secs(15);
const FULL_SYNCHRONIZATION_INTERVAL: Duration = Duration::from_secs(30);
const RESOURCE_SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

pub struct DockerRuntimeSupervisor {
    client: Mutex<Option<Docker>>,
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
        Self {
            client: Mutex::new(None),
            reconciliation_lock: Mutex::new(()),
            state: RwLock::new(DockerRuntimeState {
                snapshot: empty_snapshot(),
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

        Ok(build_snapshot(containers, resources.as_ref(), previous))
    }

    async fn publish_current(&self, publish: &SnapshotPublisher, refresh_tray: bool) {
        publish(self.state.read().await.snapshot.clone(), refresh_tray);
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
        self.state.write().await.snapshot.connection = connection_status(error);
    }

    async fn execute_read_only_operation(
        &self,
        operation: RuntimeOperation,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        let _reconciliation_guard = self.reconciliation_lock.lock().await;
        let mut state = self.state.write().await;

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
                service.bulk_selected = selected;
            }
            RuntimeOperation::SetLanguage(language) => {
                state.snapshot.preferences.language = language;
            }
            RuntimeOperation::SetProjectActive { active, project_id } => {
                let project = state
                    .snapshot
                    .projects
                    .iter_mut()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| RuntimeError::new("PROJECT_NOT_FOUND", false))?;
                project.active = active;
            }
            RuntimeOperation::SetTheme(theme) => {
                state.snapshot.preferences.theme = theme;
            }
            RuntimeOperation::RunProjectAction { action, project_id } => {
                let _ = (action, project_id);
                return Err(RuntimeError::new("RUNTIME_READ_ONLY", false));
            }
            RuntimeOperation::RunServiceAction { action, service_id } => {
                let _ = (action, service_id);
                return Err(RuntimeError::new("RUNTIME_READ_ONLY", false));
            }
            RuntimeOperation::StopAll => {
                return Err(RuntimeError::new("RUNTIME_READ_ONLY", false));
            }
        }

        Ok(state.snapshot.clone())
    }
}

impl RuntimeSupervisor for DockerRuntimeSupervisor {
    fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move { self.execute_read_only_operation(operation).await })
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
            lifecycle_actions: false,
            logs: false,
        },
        connection: ConnectionStatus::Connected,
        global_stop_in_progress: false,
        preferences: previous.preferences.clone(),
        projects,
        standalone_containers,
    }
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
        memory_bytes,
        name,
        ports,
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
                || left.services.len() != right.services.len()
                || left
                    .services
                    .iter()
                    .zip(&right.services)
                    .any(|(left, right)| left.id != right.id || left.status != right.status)
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

        let next = build_snapshot(containers, None, &previous);

        assert!(next.projects[0].active);
        assert!(!next.projects[0].services[0].bulk_selected);
    }
}
