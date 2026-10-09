use crate::runtime::{
    ApplicationSnapshot, RuntimeError, RuntimeOperation, RuntimeSupervisor, ServiceLogSnapshot,
    TestRuntimeSupervisor,
};
use std::sync::Arc;

pub struct StateCoordinator {
    runtime: Arc<dyn RuntimeSupervisor>,
}

impl StateCoordinator {
    pub fn test() -> Self {
        Self {
            runtime: Arc::new(TestRuntimeSupervisor::new()),
        }
    }

    pub fn execute(
        &self,
        operation: RuntimeOperation,
    ) -> Result<ApplicationSnapshot, RuntimeError> {
        self.runtime.execute(operation)
    }

    pub fn logs(&self, service_id: &str) -> Result<ServiceLogSnapshot, RuntimeError> {
        self.runtime.logs(service_id)
    }

    pub fn snapshot(&self) -> Result<ApplicationSnapshot, RuntimeError> {
        self.runtime.snapshot()
    }
}
