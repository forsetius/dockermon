use serde::Serialize;
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationStatus {
    stage: u8,
    shell_ready: bool,
}

#[tauri::command]
fn application_status() -> ApplicationStatus {
    ApplicationStatus {
        stage: 0,
        shell_ready: true,
    }
}

fn show_main_window(app: &tauri::AppHandle) {
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
        .invoke_handler(tauri::generate_handler![application_status])
        .setup(|app| {
            let open_item = MenuItem::with_id(app, "open", "Open Dockermon", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .expect("the application icon should be configured")
                        .clone(),
                )
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

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

#[cfg(test)]
mod tests {
    use super::application_status;

    #[test]
    fn reports_the_stage_zero_shell_as_ready() {
        let status = application_status();

        assert_eq!(status.stage, 0);
        assert!(status.shell_ready);
    }
}
