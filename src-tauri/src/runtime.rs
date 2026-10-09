use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
const MEGABYTE: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub language: Locale,
    pub theme: ThemePreference,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Pl,
}

impl Locale {
    pub(crate) fn system() -> Self {
        let locale = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default();
        if locale.to_ascii_lowercase().starts_with("pl") {
            Self::Pl
        } else {
            Self::En
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    #[serde(rename = "permission-denied")]
    PermissionDenied,
    Disconnected,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceStatus {
    NotCreated,
    Stopped,
    Starting,
    Running,
    Healthy,
    Unhealthy,
    Paused,
    Error,
}

impl ServiceStatus {
    pub fn is_ready(self) -> bool {
        matches!(self, Self::Running | Self::Healthy)
    }

    pub(crate) fn can_stop(self) -> bool {
        matches!(
            self,
            Self::Running | Self::Healthy | Self::Unhealthy | Self::Paused | Self::Starting
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Resume,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectAction {
    StartSelected,
    StopSelected,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSnapshot {
    pub bulk_selected: bool,
    pub cpu_percent: Option<f64>,
    pub id: String,
    pub memory_bytes: Option<u64>,
    pub name: String,
    pub ports: Vec<String>,
    pub status: ServiceStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub active: bool,
    pub id: String,
    pub name: String,
    pub services: Vec<ServiceSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub action: String,
    pub id: String,
    pub occurred_at: String,
    pub subject: String,
    pub successful: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub lifecycle_actions: bool,
    pub logs: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationSnapshot {
    pub activity: Vec<ActivityEntry>,
    pub capabilities: RuntimeCapabilities,
    pub connection: ConnectionStatus,
    pub global_stop_in_progress: bool,
    pub preferences: Preferences,
    pub projects: Vec<ProjectSnapshot>,
    pub standalone_containers: Vec<ServiceSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceLogSnapshot {
    pub lines: Vec<String>,
    pub service_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub code: String,
    pub retryable: bool,
}

impl RuntimeError {
    pub fn new(code: &str, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            retryable,
        }
    }

    #[cfg(test)]
    fn not_found(code: &str) -> Self {
        Self::new(code, false)
    }
}

pub enum RuntimeOperation {
    RunProjectAction {
        action: ProjectAction,
        project_id: String,
    },
    RunServiceAction {
        action: ServiceAction,
        service_id: String,
    },
    SetBulkSelected {
        project_id: String,
        selected: bool,
        service_id: String,
    },
    SetLanguage(Locale),
    SetProjectActive {
        active: bool,
        project_id: String,
    },
    SetTheme(ThemePreference),
    StopAll,
}

pub type SnapshotPublisher = Arc<dyn Fn(ApplicationSnapshot, bool) + Send + Sync>;
pub type VisibilityProbe = Arc<dyn Fn() -> bool + Send + Sync>;

pub trait RuntimeSupervisor: Send + Sync {
    fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>>;
    fn logs<'a>(
        &'a self,
        service_id: &'a str,
    ) -> BoxFuture<'a, Result<ServiceLogSnapshot, RuntimeError>>;
    fn snapshot(&self) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>>;
    fn start_monitoring(
        self: Arc<Self>,
        publish: SnapshotPublisher,
        window_is_visible: VisibilityProbe,
    );
    fn synchronize(
        &self,
        include_resources: bool,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>>;
}

#[cfg(test)]
pub struct TestRuntimeSupervisor {
    state: Mutex<TestRuntimeState>,
}

#[cfg(test)]
struct TestRuntimeState {
    activity_sequence: u64,
    snapshot: ApplicationSnapshot,
}

#[cfg(test)]
impl TestRuntimeSupervisor {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(TestRuntimeState {
                activity_sequence: 2,
                snapshot: fixture_snapshot(),
            }),
        }
    }

    fn state(&self) -> Result<std::sync::MutexGuard<'_, TestRuntimeState>, RuntimeError> {
        self.state.lock().map_err(|_| RuntimeError {
            code: "MOCK_STATE_POISONED".to_owned(),
            retryable: true,
        })
    }
}

#[cfg(test)]
impl RuntimeSupervisor for TestRuntimeSupervisor {
    fn snapshot(&self) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move { Ok(self.state()?.snapshot.clone()) })
    }

    fn logs<'a>(
        &'a self,
        service_id: &'a str,
    ) -> BoxFuture<'a, Result<ServiceLogSnapshot, RuntimeError>> {
        Box::pin(async move {
            let state = self.state()?;
            if find_service(&state.snapshot, service_id).is_none() {
                return Err(RuntimeError::not_found("MOCK_SERVICE_NOT_FOUND"));
            }

            let lines = if service_id == "api-local-api" {
                vec![
                    "[12:36:14] [INFO] Starting api service...",
                    "[12:36:15] [INFO] Loading environment from .env",
                    "[12:36:15] [INFO] Connecting to database...",
                    "[12:36:16] [INFO] Database connection ready",
                    "[12:36:16] [INFO] Running migrations...",
                    "[12:36:17] [INFO] Migrations completed",
                    "[12:36:17] [INFO] Server listening on :3000",
                    "[12:36:21] [INFO] GET /api/health 200",
                    "[12:36:28] [INFO] GET /api/users 200",
                    "[12:36:32] [INFO] POST /api/login 201",
                    "[12:36:45] [INFO] GET /api/health 200",
                ]
            } else {
                vec![
                    "[12:36:14] [INFO] Container started",
                    "[12:36:15] [INFO] Waiting for application logs...",
                ]
            };

            Ok(ServiceLogSnapshot {
                lines: lines.into_iter().map(str::to_owned).collect(),
                service_id: service_id.to_owned(),
            })
        })
    }

    fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        Box::pin(async move {
            let mut state = self.state()?;
            let mut activity: Option<(String, String)> = None;

            match operation {
                RuntimeOperation::RunProjectAction { action, project_id } => {
                    let project = state
                        .snapshot
                        .projects
                        .iter_mut()
                        .find(|project| project.id == project_id)
                        .ok_or_else(|| RuntimeError::not_found("MOCK_PROJECT_NOT_FOUND"))?;

                    for service in project
                        .services
                        .iter_mut()
                        .filter(|service| service.bulk_selected)
                    {
                        set_service_state(
                            service,
                            match action {
                                ProjectAction::StartSelected => ServiceAction::Start,
                                ProjectAction::StopSelected => ServiceAction::Stop,
                            },
                        );
                    }
                    activity = Some((
                        match action {
                            ProjectAction::StartSelected => "start-selected",
                            ProjectAction::StopSelected => "stop-selected",
                        }
                        .to_owned(),
                        project.name.clone(),
                    ));
                }
                RuntimeOperation::RunServiceAction { action, service_id } => {
                    let service = find_service_mut(&mut state.snapshot, &service_id)
                        .ok_or_else(|| RuntimeError::not_found("MOCK_SERVICE_NOT_FOUND"))?;
                    set_service_state(service, action);
                    activity = Some((
                        match action {
                            ServiceAction::Start => "start",
                            ServiceAction::Stop => "stop",
                            ServiceAction::Restart => "restart",
                            ServiceAction::Resume => "resume",
                        }
                        .to_owned(),
                        service.name.clone(),
                    ));
                }
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
                        .ok_or_else(|| RuntimeError::not_found("MOCK_PROJECT_NOT_FOUND"))?;
                    let service = project
                        .services
                        .iter_mut()
                        .find(|service| service.id == service_id)
                        .ok_or_else(|| RuntimeError::not_found("MOCK_SERVICE_NOT_FOUND"))?;
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
                        .ok_or_else(|| RuntimeError::not_found("MOCK_PROJECT_NOT_FOUND"))?;
                    project.active = active;
                }
                RuntimeOperation::SetTheme(theme) => {
                    state.snapshot.preferences.theme = theme;
                }
                RuntimeOperation::StopAll => {
                    state.snapshot.global_stop_in_progress = true;
                    for project in &mut state.snapshot.projects {
                        for service in &mut project.services {
                            if service.status.can_stop() {
                                set_service_state(service, ServiceAction::Stop);
                            }
                        }
                    }
                    for service in &mut state.snapshot.standalone_containers {
                        if service.status.can_stop() {
                            set_service_state(service, ServiceAction::Stop);
                        }
                    }
                    state.snapshot.global_stop_in_progress = false;
                    activity = Some(("stop-all".to_owned(), "Docker Engine".to_owned()));
                }
            }

            if let Some((action, subject)) = activity {
                state.activity_sequence += 1;
                let activity_id = format!("activity-{}", state.activity_sequence);
                state.snapshot.activity.insert(
                    0,
                    ActivityEntry {
                        action,
                        id: activity_id,
                        occurred_at: "12:40:00".to_owned(),
                        subject,
                        successful: true,
                    },
                );
            }

            Ok(state.snapshot.clone())
        })
    }

    fn start_monitoring(
        self: Arc<Self>,
        _publish: SnapshotPublisher,
        _window_is_visible: VisibilityProbe,
    ) {
    }

    fn synchronize(
        &self,
        _include_resources: bool,
    ) -> BoxFuture<'_, Result<ApplicationSnapshot, RuntimeError>> {
        self.snapshot()
    }
}

#[cfg(test)]
fn set_service_state(service: &mut ServiceSnapshot, action: ServiceAction) {
    match action {
        ServiceAction::Stop => {
            service.status = ServiceStatus::Stopped;
            service.cpu_percent = None;
            service.memory_bytes = None;
        }
        ServiceAction::Start | ServiceAction::Restart | ServiceAction::Resume => {
            service.status = ServiceStatus::Running;
            service.cpu_percent = Some(service.cpu_percent.unwrap_or(0.1).max(0.1));
            service.memory_bytes = Some(service.memory_bytes.unwrap_or(32 * MEGABYTE));
        }
    }
}

#[cfg(test)]
fn find_service<'a>(
    snapshot: &'a ApplicationSnapshot,
    service_id: &str,
) -> Option<&'a ServiceSnapshot> {
    snapshot
        .projects
        .iter()
        .flat_map(|project| &project.services)
        .chain(&snapshot.standalone_containers)
        .find(|service| service.id == service_id)
}

#[cfg(test)]
fn find_service_mut<'a>(
    snapshot: &'a mut ApplicationSnapshot,
    service_id: &str,
) -> Option<&'a mut ServiceSnapshot> {
    snapshot
        .projects
        .iter_mut()
        .flat_map(|project| &mut project.services)
        .chain(&mut snapshot.standalone_containers)
        .find(|service| service.id == service_id)
}

#[cfg(test)]
fn service(
    id: &str,
    name: &str,
    status: ServiceStatus,
    bulk_selected: bool,
    cpu_percent: Option<f64>,
    memory_megabytes: Option<u64>,
    ports: &[&str],
) -> ServiceSnapshot {
    ServiceSnapshot {
        bulk_selected,
        cpu_percent,
        id: id.to_owned(),
        memory_bytes: memory_megabytes.map(|value| value * MEGABYTE),
        name: name.to_owned(),
        ports: ports.iter().map(|port| (*port).to_owned()).collect(),
        status,
    }
}

#[cfg(test)]
fn fixture_snapshot() -> ApplicationSnapshot {
    ApplicationSnapshot {
        activity: vec![
            ActivityEntry {
                action: "restart".to_owned(),
                id: "activity-1".to_owned(),
                occurred_at: "12:31:04".to_owned(),
                subject: "api-local/api".to_owned(),
                successful: true,
            },
            ActivityEntry {
                action: "stop".to_owned(),
                id: "activity-2".to_owned(),
                occurred_at: "11:58:20".to_owned(),
                subject: "api-local/mailpit".to_owned(),
                successful: true,
            },
        ],
        capabilities: RuntimeCapabilities {
            lifecycle_actions: true,
            logs: true,
        },
        connection: ConnectionStatus::Connected,
        global_stop_in_progress: false,
        preferences: Preferences {
            language: Locale::system(),
            theme: ThemePreference::System,
        },
        projects: vec![
            ProjectSnapshot {
                active: true,
                id: "storefront".to_owned(),
                name: "Sklep lokalny".to_owned(),
                services: vec![
                    service(
                        "storefront-web",
                        "web",
                        ServiceStatus::Running,
                        true,
                        Some(1.8),
                        Some(284),
                        &["3000:3000"],
                    ),
                    service(
                        "storefront-api",
                        "api",
                        ServiceStatus::Healthy,
                        false,
                        Some(0.2),
                        Some(96),
                        &["3001:3000"],
                    ),
                    service(
                        "storefront-postgres",
                        "postgres",
                        ServiceStatus::Running,
                        true,
                        Some(1.1),
                        Some(512),
                        &["5432:5432"],
                    ),
                    service(
                        "storefront-redis",
                        "redis",
                        ServiceStatus::Running,
                        false,
                        Some(0.4),
                        Some(128),
                        &["6379:6379"],
                    ),
                ],
            },
            ProjectSnapshot {
                active: true,
                id: "api-local".to_owned(),
                name: "API lokalne".to_owned(),
                services: vec![
                    service(
                        "api-local-web",
                        "web",
                        ServiceStatus::Running,
                        false,
                        Some(0.7),
                        Some(180),
                        &["8080:3000"],
                    ),
                    service(
                        "api-local-api",
                        "api",
                        ServiceStatus::Healthy,
                        true,
                        Some(1.2),
                        Some(256),
                        &["3000:3000"],
                    ),
                    service(
                        "api-local-postgres",
                        "postgres",
                        ServiceStatus::Stopped,
                        false,
                        None,
                        None,
                        &["5432:5432"],
                    ),
                    service(
                        "api-local-redis",
                        "redis",
                        ServiceStatus::Starting,
                        false,
                        Some(0.0),
                        Some(12),
                        &["6379:6379"],
                    ),
                    service(
                        "api-local-mailpit",
                        "mailpit",
                        ServiceStatus::Stopped,
                        false,
                        None,
                        None,
                        &["8025:8025"],
                    ),
                ],
            },
            ProjectSnapshot {
                active: false,
                id: "docs-preview".to_owned(),
                name: "Dokumentacja".to_owned(),
                services: vec![service(
                    "docs-preview-site",
                    "site",
                    ServiceStatus::Stopped,
                    true,
                    None,
                    None,
                    &["4173:4173"],
                )],
            },
        ],
        standalone_containers: vec![
            service(
                "standalone-minio",
                "local-minio",
                ServiceStatus::Running,
                false,
                Some(0.1),
                Some(148),
                &["9000:9000", "9001:9001"],
            ),
            service(
                "standalone-mailhog",
                "legacy-mailhog",
                ServiceStatus::Stopped,
                false,
                None,
                None,
                &["8026:8025"],
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_action_only_changes_selected_services_in_one_project() {
        let supervisor = TestRuntimeSupervisor::new();

        let snapshot = run(supervisor.execute(RuntimeOperation::RunProjectAction {
            action: ProjectAction::StopSelected,
            project_id: "storefront".to_owned(),
        }))
        .expect("test operation should succeed");

        assert_eq!(
            snapshot.projects[0].services[0].status,
            ServiceStatus::Stopped
        );
        assert_eq!(
            snapshot.projects[0].services[1].status,
            ServiceStatus::Healthy
        );
        assert_eq!(
            snapshot.projects[1].services[1].status,
            ServiceStatus::Healthy
        );
    }

    #[test]
    fn stop_all_changes_projects_and_standalone_containers() {
        let supervisor = TestRuntimeSupervisor::new();

        let snapshot = run(supervisor.execute(RuntimeOperation::StopAll))
            .expect("test operation should succeed");

        assert!(
            snapshot
                .projects
                .iter()
                .flat_map(|project| &project.services)
                .chain(&snapshot.standalone_containers)
                .all(|service| !service.status.can_stop())
        );
    }

    #[test]
    fn resume_changes_a_paused_service_to_running() {
        let mut paused_service = service(
            "paused-worker",
            "worker",
            ServiceStatus::Paused,
            false,
            Some(0.0),
            Some(32),
            &[],
        );

        set_service_state(&mut paused_service, ServiceAction::Resume);

        assert_eq!(paused_service.status, ServiceStatus::Running);
    }

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(future)
    }
}
