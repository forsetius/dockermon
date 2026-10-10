mod docker_runtime;
mod project_catalog;
mod runtime;
mod state;
mod tray;

use runtime::{
    ApplicationSnapshot, Locale, LogPublisher, ProjectAction, RuntimeError, RuntimeOperation,
    ServiceAction, ServiceLogBatch, ServiceLogSnapshot, ThemePreference,
};
use state::StateCoordinator;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

#[tauri::command]
async fn application_snapshot(
    coordinator: State<'_, StateCoordinator>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    coordinator.snapshot().await
}

#[tauri::command]
async fn start_service_logs(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    service_id: String,
    subscription_id: String,
) -> Result<ServiceLogSnapshot, RuntimeError> {
    let publisher_app = app.clone();
    let publish: LogPublisher = Arc::new(move |batch: ServiceLogBatch| {
        let _ = publisher_app.emit("service-log-batch", batch);
    });
    coordinator
        .start_logs(&service_id, &subscription_id, publish)
        .await
}

#[tauri::command]
async fn stop_service_logs(
    coordinator: State<'_, StateCoordinator>,
    subscription_id: String,
) -> Result<(), RuntimeError> {
    coordinator.stop_logs(&subscription_id).await;
    Ok(())
}

#[tauri::command]
async fn run_project_action(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    project_id: String,
    action: ProjectAction,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::RunProjectAction { action, project_id },
    )
    .await
}

#[tauri::command]
async fn run_service_action(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    service_id: String,
    action: ServiceAction,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::RunServiceAction { action, service_id },
    )
    .await
}

#[tauri::command]
async fn import_project(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    paths: Vec<String>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::ImportProject { paths },
    )
    .await
}

#[tauri::command]
async fn remove_project(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    project_id: String,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::RemoveProject { project_id },
    )
    .await
}

#[tauri::command]
async fn set_bulk_selected(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    project_id: String,
    service_id: String,
    selected: bool,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::SetBulkSelected {
            project_id,
            selected,
            service_id,
        },
    )
    .await
}

#[tauri::command]
async fn set_project_active(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    project_id: String,
    active: bool,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::SetProjectActive { active, project_id },
    )
    .await
}

#[tauri::command]
async fn set_project_profiles(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    project_id: String,
    profiles: Vec<String>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(
        &app,
        &coordinator,
        RuntimeOperation::SetProjectProfiles {
            profiles,
            project_id,
        },
    )
    .await
}

#[tauri::command]
async fn set_language(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    language: Locale,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::SetLanguage(language)).await
}

#[tauri::command]
async fn set_theme(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    theme: ThemePreference,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::SetTheme(theme)).await
}

#[tauri::command]
async fn stop_all(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::StopAll).await
}

async fn apply_operation<R: Runtime>(
    app: &AppHandle<R>,
    coordinator: &StateCoordinator,
    operation: RuntimeOperation,
) -> Result<ApplicationSnapshot, RuntimeError> {
    let snapshot = coordinator.execute(operation).await?;
    let _ = tray::refresh_tray(app, &snapshot);
    let _ = app.emit("state-changed", snapshot.clone());
    Ok(snapshot)
}

pub(crate) fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                show_main_window(app);
            },
        ))
        .manage(StateCoordinator::docker())
        .invoke_handler(tauri::generate_handler![
            application_snapshot,
            import_project,
            remove_project,
            run_project_action,
            run_service_action,
            start_service_logs,
            stop_service_logs,
            set_bulk_selected,
            set_language,
            set_project_active,
            set_project_profiles,
            set_theme,
            stop_all,
        ])
        .setup(|app| {
            let coordinator = app.state::<StateCoordinator>().inner().clone();
            let snapshot = tauri::async_runtime::block_on(async {
                let _ = coordinator.synchronize(true).await;
                coordinator.snapshot().await
            })
            .map_err(|error| format!("failed to initialize Docker runtime: {}", error.code))?;
            tray::build_tray(app.handle(), &snapshot)?;

            let publisher_app = app.handle().clone();
            let publish = Arc::new(move |snapshot: ApplicationSnapshot, refresh_tray: bool| {
                if refresh_tray {
                    let _ = tray::refresh_tray(&publisher_app, &snapshot);
                }
                let _ = publisher_app.emit("state-changed", snapshot);
            });
            let visibility_app = app.handle().clone();
            let window_is_visible = Arc::new(move || {
                visibility_app
                    .get_webview_window("main")
                    .and_then(|window| window.is_visible().ok())
                    .unwrap_or(false)
            });
            coordinator.start_monitoring(publish, window_is_visible);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run Dockermon");
}
