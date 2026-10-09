use crate::{
    runtime::{
        ApplicationSnapshot, Locale, RuntimeOperation, ServiceAction, ServiceSnapshot,
        ServiceStatus,
    },
    show_main_window,
    state::StateCoordinator,
};
use tauri::{
    AppHandle, Emitter, Manager, Runtime,
    image::Image,
    menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
};

const TRAY_ID: &str = "main";

pub fn build_tray<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &ApplicationSnapshot,
) -> tauri::Result<()> {
    let menu = create_menu(app, snapshot)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon(app, snapshot))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    Ok(())
}

pub fn refresh_tray<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &ApplicationSnapshot,
) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(create_menu(app, snapshot)?))?;
        tray.set_icon(Some(tray_icon(app, snapshot)))?;
    }
    Ok(())
}

fn tray_icon<R: Runtime>(app: &AppHandle<R>, snapshot: &ApplicationSnapshot) -> Image<'static> {
    let source = app
        .default_window_icon()
        .expect("the application icon should be configured");
    let mut rgba = source.rgba().to_vec();
    if !snapshot.projects.iter().any(|project| project.active) {
        let (pixels, _) = rgba.as_chunks_mut::<4>();
        for pixel in pixels {
            let luminance =
                ((u16::from(pixel[0]) * 54 + u16::from(pixel[1]) * 183 + u16::from(pixel[2]) * 19)
                    / 256) as u8;
            pixel[0] = luminance;
            pixel[1] = luminance;
            pixel[2] = luminance;
        }
    }
    Image::new_owned(rgba, source.width(), source.height())
}

fn create_menu<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &ApplicationSnapshot,
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    let projects = visible_projects(snapshot);

    for project in projects {
        let ready_count = project
            .services
            .iter()
            .filter(|service| service.included && service.status.is_ready())
            .count();
        let included_count = project
            .services
            .iter()
            .filter(|service| service.included)
            .count();
        let project_menu = Submenu::new(
            app,
            format!("{} {}/{}", project.name, ready_count, included_count),
            true,
        )?;

        for service in project.services.iter().filter(|service| service.included) {
            append_service_action(
                app,
                &project_menu,
                service,
                snapshot.capabilities.lifecycle_actions && !snapshot.global_stop_in_progress,
            )?;
        }

        menu.append(&project_menu)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "global-stop",
        text(
            snapshot.preferences.language,
            "Stop all",
            "Zatrzymaj wszystko",
        ),
        snapshot.capabilities.lifecycle_actions && !snapshot.global_stop_in_progress,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        text(snapshot.preferences.language, "Open window", "Otwórz okno"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        text(snapshot.preferences.language, "Quit", "Zakończ"),
        true,
        None::<&str>,
    )?)?;

    Ok(menu)
}

fn append_service_action<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Submenu<R>,
    service: &ServiceSnapshot,
    enabled: bool,
) -> tauri::Result<()> {
    let action = service_action(service.status);
    menu.append(&IconMenuItem::with_id(
        app,
        format!("service|{}|{}", service.id, action_id(action)),
        &service.name,
        enabled,
        Some(action_icon(action)),
        None::<&str>,
    )?)?;

    Ok(())
}

fn service_action(status: ServiceStatus) -> ServiceAction {
    match status {
        ServiceStatus::NotCreated | ServiceStatus::Stopped | ServiceStatus::Error => {
            ServiceAction::Start
        }
        ServiceStatus::Starting | ServiceStatus::Running | ServiceStatus::Healthy => {
            ServiceAction::Stop
        }
        ServiceStatus::Unhealthy => ServiceAction::Restart,
        ServiceStatus::Paused => ServiceAction::Resume,
    }
}

fn action_id(action: ServiceAction) -> &'static str {
    match action {
        ServiceAction::Start => "start",
        ServiceAction::Stop => "stop",
        ServiceAction::Restart => "restart",
        ServiceAction::Resume => "resume",
    }
}

fn action_icon(action: ServiceAction) -> Image<'static> {
    const ICON_SIZE: usize = 16;
    const START_ICON: [&str; ICON_SIZE] = [
        "................",
        "................",
        ".....#..........",
        ".....##.........",
        ".....###........",
        ".....####.......",
        ".....#####......",
        ".....######.....",
        ".....######.....",
        ".....#####......",
        ".....####.......",
        ".....###........",
        ".....##.........",
        ".....#..........",
        "................",
        "................",
    ];
    const STOP_ICON: [&str; ICON_SIZE] = [
        "................",
        "................",
        "................",
        "................",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "................",
        "................",
        "................",
        "................",
    ];
    const RESTART_ICON: [&str; ICON_SIZE] = [
        "................",
        "................",
        "......####......",
        "....##....##....",
        "...##......##...",
        "..##........##..",
        "..##.....#####..",
        "..##......####..",
        "..##........##..",
        "...##......##...",
        "....##....##....",
        "......####......",
        "................",
        "................",
        "................",
        "................",
    ];

    let (mask, color) = match action {
        ServiceAction::Start | ServiceAction::Resume => (&START_ICON, [45, 145, 248, 255]),
        ServiceAction::Stop => (&STOP_ICON, [255, 82, 94, 255]),
        ServiceAction::Restart => (&RESTART_ICON, [45, 145, 248, 255]),
    };
    let mut rgba = vec![0; ICON_SIZE * ICON_SIZE * 4];

    for (y, row) in mask.iter().enumerate() {
        for (x, pixel) in row.bytes().enumerate() {
            if pixel == b'#' {
                let offset = (y * ICON_SIZE + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }

    Image::new_owned(rgba, ICON_SIZE as u32, ICON_SIZE as u32)
}

fn visible_projects(snapshot: &ApplicationSnapshot) -> Vec<&crate::runtime::ProjectSnapshot> {
    if snapshot.projects.iter().any(|project| project.active) {
        snapshot
            .projects
            .iter()
            .filter(|project| project.active)
            .collect()
    } else {
        snapshot.projects.iter().collect()
    }
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "open" => show_main_window(app),
        "quit" => app.exit(0),
        "global-stop" => apply_tray_operation(app, RuntimeOperation::StopAll),
        id if id.starts_with("service|") => {
            let parts: Vec<_> = id.split('|').collect();
            if let ["service", service_id, action] = parts.as_slice() {
                let action = match *action {
                    "start" => Some(ServiceAction::Start),
                    "stop" => Some(ServiceAction::Stop),
                    "restart" => Some(ServiceAction::Restart),
                    "resume" => Some(ServiceAction::Resume),
                    _ => None,
                };
                if let Some(action) = action {
                    apply_tray_operation(
                        app,
                        RuntimeOperation::RunServiceAction {
                            action,
                            service_id: (*service_id).to_owned(),
                        },
                    );
                }
            }
        }
        _ => {}
    }
}

fn apply_tray_operation<R: Runtime>(app: &AppHandle<R>, operation: RuntimeOperation) {
    let coordinator = app.state::<StateCoordinator>().inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(snapshot) = coordinator.execute(operation).await {
            let _ = refresh_tray(&app, &snapshot);
            let _ = app.emit("state-changed", snapshot);
        }
    });
}

fn text<'a>(language: Locale, english: &'a str, polish: &'a str) -> &'a str {
    match language {
        Locale::En => english,
        Locale::Pl => polish,
    }
}

#[cfg(test)]
mod tests {
    use super::{service_action, visible_projects};
    use crate::runtime::{RuntimeSupervisor, ServiceAction, ServiceStatus, TestRuntimeSupervisor};

    #[test]
    fn tray_assigns_exactly_one_action_to_each_service_status() {
        assert_eq!(
            service_action(ServiceStatus::NotCreated),
            ServiceAction::Start
        );
        assert_eq!(service_action(ServiceStatus::Stopped), ServiceAction::Start);
        assert_eq!(service_action(ServiceStatus::Starting), ServiceAction::Stop);
        assert_eq!(service_action(ServiceStatus::Running), ServiceAction::Stop);
        assert_eq!(service_action(ServiceStatus::Healthy), ServiceAction::Stop);
        assert_eq!(
            service_action(ServiceStatus::Unhealthy),
            ServiceAction::Restart
        );
        assert_eq!(service_action(ServiceStatus::Paused), ServiceAction::Resume);
        assert_eq!(service_action(ServiceStatus::Error), ServiceAction::Start);
    }

    #[test]
    fn tray_uses_only_active_projects_when_any_are_active() {
        let supervisor = TestRuntimeSupervisor::new();
        let snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");

        let visible: Vec<_> = visible_projects(&snapshot)
            .into_iter()
            .map(|project| project.id.as_str())
            .collect();

        assert_eq!(visible, vec!["storefront", "api-local"]);
    }

    #[test]
    fn tray_uses_all_projects_when_none_are_active() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");
        for project in &mut snapshot.projects {
            project.active = false;
        }

        assert_eq!(visible_projects(&snapshot).len(), snapshot.projects.len());
    }

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(future)
    }
}
