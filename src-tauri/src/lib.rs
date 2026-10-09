mod runtime;
mod state;
mod tray;

use runtime::{
    ApplicationSnapshot, Locale, ProjectAction, RuntimeError, RuntimeOperation, ServiceAction,
    ServiceLogSnapshot, ThemePreference,
};
use state::StateCoordinator;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

#[tauri::command]
fn application_snapshot(
    coordinator: State<'_, StateCoordinator>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    coordinator.snapshot()
}

#[tauri::command]
fn service_logs(
    coordinator: State<'_, StateCoordinator>,
    service_id: String,
) -> Result<ServiceLogSnapshot, RuntimeError> {
    coordinator.logs(&service_id)
}

#[tauri::command]
fn run_project_action(
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
}

#[tauri::command]
fn run_service_action(
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
}

#[tauri::command]
fn set_bulk_selected(
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
}

#[tauri::command]
fn set_project_active(
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
}

#[tauri::command]
fn set_language(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    language: Locale,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::SetLanguage(language))
}

#[tauri::command]
fn set_theme(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    theme: ThemePreference,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::SetTheme(theme))
}

#[tauri::command]
fn stop_all(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
) -> Result<ApplicationSnapshot, RuntimeError> {
    apply_operation(&app, &coordinator, RuntimeOperation::StopAll)
}

fn apply_operation<R: Runtime>(
    app: &AppHandle<R>,
    coordinator: &StateCoordinator,
    operation: RuntimeOperation,
) -> Result<ApplicationSnapshot, RuntimeError> {
    let snapshot = coordinator.execute(operation)?;
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
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                show_main_window(app);
            },
        ))
        .manage(StateCoordinator::test())
        .invoke_handler(tauri::generate_handler![
            application_snapshot,
            run_project_action,
            run_service_action,
            service_logs,
            set_bulk_selected,
            set_language,
            set_project_active,
            set_theme,
            stop_all,
        ])
        .setup(|app| {
            let snapshot = app
                .state::<StateCoordinator>()
                .snapshot()
                .map_err(|error| format!("failed to initialize test runtime: {}", error.code))?;
            tray::build_tray(app.handle(), &snapshot)?;
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
