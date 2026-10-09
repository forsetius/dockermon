use crate::docker_runtime::DockerRuntimeSupervisor;
use crate::runtime::{
    ApplicationSnapshot, RuntimeError, RuntimeOperation, RuntimeSupervisor, ServiceLogSnapshot,
    SnapshotPublisher, VisibilityProbe,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct StateCoordinator {
    runtime: Arc<dyn RuntimeSupervisor>,
}

impl StateCoordinator {
    pub fn docker() -> Self {
        Self {
            runtime: Arc::new(DockerRuntimeSupervisor::new()),
        }
    }

    pub async fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        self.runtime.execute(operation).await
    }

    pub async fn logs(&self, service_id: &str) -> Result<ServiceLogSnapshot, RuntimeError> {
        self.runtime.logs(service_id).await
    }

    pub async fn snapshot(&self) -> Result<ApplicationSnapshot, RuntimeError> {
        self.runtime.snapshot().await
    }

    pub fn start_monitoring(&self, publish: SnapshotPublisher, window_is_visible: VisibilityProbe) {
        self.runtime
            .clone()
            .start_monitoring(publish, window_is_visible);
    }

    pub async fn synchronize(
        &self,
        include_resources: bool,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        self.runtime.synchronize(include_resources).await
    }
}
