mod docker_runtime;
mod project_catalog;
mod runtime;
mod state;
mod tray;

use auto_launch::{AutoLaunch, LinuxLaunchMode};
use runtime::{
    ApplicationSnapshot, Locale, LogPublisher, ProjectAction, RuntimeError, RuntimeOperation,
    ServiceAction, ServiceLogBatch, ServiceLogSnapshot, ThemePreference,
};
use state::StateCoordinator;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

const AUTOSTART_ARGUMENT: &str = "--autostart";
const AUTOSTART_APPLICATION_NAME: &str = "Dockermon";
const INSTALLED_EXECUTABLE_PATH: &str = "/usr/bin/dockermon";

fn autostart_registration() -> AutoLaunch {
    AutoLaunch::new(
        AUTOSTART_APPLICATION_NAME,
        INSTALLED_EXECUTABLE_PATH,
        LinuxLaunchMode::XdgAutostart,
        &[AUTOSTART_ARGUMENT],
    )
}

fn arguments_request_hidden_start<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    arguments
        .into_iter()
        .any(|argument| argument.as_ref() == std::ffi::OsStr::new(AUTOSTART_ARGUMENT))
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn development_desktop_entry_path(identifier: &str) -> Result<std::path::PathBuf, std::io::Error> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".local/share"))
        })
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "XDG_DATA_HOME and HOME are unavailable",
            )
        })?;

    Ok(data_home
        .join("applications")
        .join(format!("{identifier}.desktop")))
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn quote_desktop_exec_argument(path: &std::path::Path) -> String {
    let escaped = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{escaped}\"")
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn development_desktop_entry_contents(
    identifier: &str,
    executable: &std::path::Path,
    icon: &std::path::Path,
) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Dockermon\nExec={}\nIcon={}\nTerminal=false\nNoDisplay=true\nStartupWMClass={}\n",
        quote_desktop_exec_argument(executable),
        icon.display(),
        identifier,
    )
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn install_development_desktop_entry(
    identifier: &str,
) -> Result<std::path::PathBuf, std::io::Error> {
    let desktop_entry_path = development_desktop_entry_path(identifier)?;
    let executable = std::env::current_exe()?;
    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/dockermon.png");
    let contents = development_desktop_entry_contents(identifier, &executable, &icon);
    if let Some(parent) = desktop_entry_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&desktop_entry_path, contents)?;
    Ok(desktop_entry_path)
}

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

fn update_autostart_registration(enabled: bool) -> Result<(), RuntimeError> {
    let registration = autostart_registration();
    let result = if enabled {
        registration.enable()
    } else {
        registration.disable()
    };
    result.map_err(|_| RuntimeError::new("AUTOSTART_UPDATE_FAILED", true))
}

#[tauri::command]
async fn set_autostart(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    enabled: bool,
) -> Result<ApplicationSnapshot, RuntimeError> {
    let previous = autostart_registration()
        .is_enabled()
        .map_err(|_| RuntimeError::new("AUTOSTART_UPDATE_FAILED", true))?;
    update_autostart_registration(enabled)?;

    match apply_operation(&app, &coordinator, RuntimeOperation::SetAutostart(enabled)).await {
        Ok(snapshot) => Ok(snapshot),
        Err(error) => {
            let _ = update_autostart_registration(previous);
            Err(error)
        }
    }
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
async fn set_system_theme(
    app: AppHandle,
    coordinator: State<'_, StateCoordinator>,
    dark: bool,
) -> Result<(), RuntimeError> {
    let snapshot = coordinator.snapshot().await?;
    let theme = if dark {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    };
    tray::refresh_tray_icon(&app, &snapshot, theme)
        .map_err(|_| RuntimeError::new("TRAY_ICON_UPDATE_FAILED", true))
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
    let context = tauri::generate_context!();
    let start_hidden = arguments_request_hidden_start(std::env::args_os());
    #[cfg(all(debug_assertions, target_os = "linux"))]
    let development_desktop_entry =
        match install_development_desktop_entry(&context.config().identifier) {
            Ok(path) => Some(path),
            Err(error) => {
                eprintln!("failed to install the development desktop entry: {error}");
                None
            }
        };

    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(
            |app, arguments, _working_directory| {
                if !arguments_request_hidden_start(&arguments) {
                    show_main_window(app);
                }
            },
        ))
        .manage(tray::TrayThemeState::default())
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
            set_autostart,
            set_language,
            set_project_active,
            set_project_profiles,
            set_system_theme,
            set_theme,
            stop_all,
        ])
        .setup(move |app| {
            if let (Some(window), Some(icon)) = (
                app.get_webview_window("main"),
                app.default_window_icon().cloned(),
            ) {
                window.set_icon(icon)?;
                if start_hidden {
                    window.hide()?;
                } else {
                    window.show()?;
                }
            }

            let coordinator = app.state::<StateCoordinator>().inner().clone();
            let autostart_registration = autostart_registration();
            let registered_autostart = autostart_registration.is_enabled().ok();
            if registered_autostart == Some(true) {
                match autostart_registration.enable() {
                    Ok(()) => {}
                    Err(error) => {
                        eprintln!("failed to refresh the autostart registration: {error}");
                    }
                }
            }
            let snapshot = tauri::async_runtime::block_on(async {
                if let Some(enabled) = registered_autostart {
                    let current = coordinator.snapshot().await?;
                    if current.preferences.autostart_enabled != enabled {
                        let _ = coordinator
                            .execute(RuntimeOperation::SetAutostart(enabled))
                            .await;
                    }
                }
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
        .run(context);

    #[cfg(all(debug_assertions, target_os = "linux"))]
    if let Some(path) = development_desktop_entry {
        let _ = std::fs::remove_file(path);
    }

    run_result.expect("failed to run Dockermon");
}

#[cfg(test)]
mod tests {
    use super::{
        AUTOSTART_APPLICATION_NAME, AUTOSTART_ARGUMENT, INSTALLED_EXECUTABLE_PATH,
        arguments_request_hidden_start, autostart_registration,
    };
    #[cfg(all(debug_assertions, target_os = "linux"))]
    use super::{development_desktop_entry_contents, development_desktop_entry_path};

    #[test]
    fn generated_context_contains_the_linux_window_identity() {
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();

        assert!(context.default_window_icon().is_some());
        assert!(context.config().app.enable_gtk_app_id);
    }

    #[test]
    fn autostart_argument_requests_a_hidden_window() {
        assert!(arguments_request_hidden_start(["dockermon", "--autostart"]));
        assert!(!arguments_request_hidden_start(["dockermon"]));
    }

    #[test]
    fn autostart_targets_the_deb_executable() {
        let registration = autostart_registration();

        assert_eq!(registration.get_app_name(), AUTOSTART_APPLICATION_NAME);
        assert_eq!(registration.get_app_path(), INSTALLED_EXECUTABLE_PATH);
        assert_eq!(registration.get_args(), &[AUTOSTART_ARGUMENT]);
    }

    #[cfg(all(debug_assertions, target_os = "linux"))]
    #[test]
    fn development_desktop_entry_matches_the_gtk_application_identifier() {
        let identifier = "pl.forsetius.dockermon";
        let path = development_desktop_entry_path(identifier)
            .expect("a desktop entry path should be available in the test environment");
        let contents = development_desktop_entry_contents(
            identifier,
            std::path::Path::new("/tmp/dockermon executable"),
            std::path::Path::new("/tmp/dockermon-icon.png"),
        );

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("pl.forsetius.dockermon.desktop")
        );
        assert!(contents.contains("Exec=\"/tmp/dockermon executable\""));
        assert!(contents.contains("Icon=/tmp/dockermon-icon.png"));
        assert!(contents.contains("StartupWMClass=pl.forsetius.dockermon"));
    }
}
