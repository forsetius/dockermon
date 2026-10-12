use crate::{
    runtime::{
        ApplicationSnapshot, ConnectionStatus, Locale, RuntimeOperation, ServiceAction,
        ServiceSnapshot, ServiceStatus, ThemePreference,
    },
    show_main_window,
    state::StateCoordinator,
};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, Runtime, Theme,
    image::Image,
    menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
};

#[derive(Default)]
pub struct TrayThemeState {
    synchronized_theme: Mutex<Option<Theme>>,
}

impl TrayThemeState {
    fn synchronize(&self, theme: Theme) {
        let mut synchronized_theme = self
            .synchronized_theme
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *synchronized_theme = Some(theme);
    }

    fn resolve(&self, reported_window_theme: Option<Theme>) -> Theme {
        self.synchronized_theme
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .copied()
            .or(reported_window_theme)
            .unwrap_or(Theme::Light)
    }
}

const TRAY_ID: &str = "main";
const TRAY_ICON_LIGHT_THEME: Image<'static> = tauri::include_image!("./icons/tray-light.png");
const TRAY_ICON_DARK_THEME: Image<'static> = tauri::include_image!("./icons/tray-dark.png");
const CONNECTED_WHALE_LIGHT_THEME: [u8; 3] = [36, 38, 51];
const CONNECTED_WHALE_DARK_THEME: [u8; 3] = [230, 237, 243];
const NEUTRAL_COLOR: [u8; 3] = [168, 180, 194];
const READY_COLOR: [u8; 3] = [43, 220, 104];
const STARTING_COLOR: [u8; 3] = [255, 179, 26];
const PROBLEM_COLOR: [u8; 3] = [255, 82, 94];
const PERMISSION_DENIED_COLOR: [u8; 3] = [192, 132, 252];
const GLOBAL_STOP_CENTER_COLOR: [u8; 3] = [255, 255, 255];
const STATUS_DOT_CENTER: (i32, i32) = (48, 48);
const GLOBAL_STOP_CENTER_RADIUS_SQUARED: i32 = 36;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayServiceState {
    Neutral,
    Ready,
    Starting,
    Problem,
    GlobalStop,
}

pub fn build_tray<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &ApplicationSnapshot,
) -> tauri::Result<()> {
    let menu = create_menu(app, snapshot)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon(
            snapshot,
            resolve_tray_theme(snapshot.preferences.theme, system_theme(app)),
        ))
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
        tray.set_icon(Some(tray_icon(
            snapshot,
            resolve_tray_theme(snapshot.preferences.theme, system_theme(app)),
        )))?;
    }
    Ok(())
}

pub fn refresh_tray_icon<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &ApplicationSnapshot,
    theme: Theme,
) -> tauri::Result<()> {
    if let Some(theme_state) = app.try_state::<TrayThemeState>() {
        theme_state.synchronize(theme);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_icon(Some(tray_icon(
            snapshot,
            resolve_tray_theme(snapshot.preferences.theme, theme),
        )))?;
    }
    Ok(())
}

fn system_theme<R: Runtime>(app: &AppHandle<R>) -> Theme {
    let reported_window_theme = app
        .get_webview_window("main")
        .and_then(|window| window.theme().ok())
        .filter(|theme| matches!(theme, Theme::Light | Theme::Dark));
    app.try_state::<TrayThemeState>().map_or_else(
        || reported_window_theme.unwrap_or(Theme::Light),
        |theme_state| theme_state.resolve(reported_window_theme),
    )
}

fn resolve_tray_theme(preference: ThemePreference, system_theme: Theme) -> Theme {
    match preference {
        ThemePreference::Light => Theme::Light,
        ThemePreference::Dark => Theme::Dark,
        ThemePreference::System => system_theme,
    }
}

fn tray_icon(snapshot: &ApplicationSnapshot, theme: Theme) -> Image<'static> {
    let source = if matches!(theme, Theme::Dark) {
        TRAY_ICON_DARK_THEME.clone()
    } else {
        TRAY_ICON_LIGHT_THEME.clone()
    };
    let mut rgba = source.rgba().to_vec();
    let connected = snapshot.connection == ConnectionStatus::Connected;
    let whale_color = connection_color(snapshot.connection, theme);
    let service_state = tray_service_state(snapshot);
    let width = usize::try_from(source.width()).expect("tray icon width should fit usize");
    let (pixels, remainder) = rgba.as_chunks_mut::<4>();
    debug_assert!(remainder.is_empty());

    for (index, pixel) in pixels.iter_mut().enumerate() {
        let x = i32::try_from(index % width).expect("tray icon x coordinate should fit i32");
        let y = i32::try_from(index / width).expect("tray icon y coordinate should fit i32");
        if !connected && hidden_whale_body_contains(x, y) {
            pixel[..3].copy_from_slice(&whale_color);
            pixel[3] = u8::MAX;
            continue;
        }

        if pixel[3] == 0 {
            continue;
        }

        let is_status_dot = is_status_dot_pixel(pixel);
        if connected && is_status_dot {
            pixel[..3].copy_from_slice(&service_color(service_state, x, y));
        } else if !connected && is_status_dot {
            pixel.fill(0);
        } else {
            pixel[..3].copy_from_slice(&whale_color);
        }
    }

    Image::new_owned(rgba, source.width(), source.height())
}

fn connection_color(connection: ConnectionStatus, theme: Theme) -> [u8; 3] {
    match connection {
        ConnectionStatus::Connected => {
            if matches!(theme, Theme::Dark) {
                CONNECTED_WHALE_DARK_THEME
            } else {
                CONNECTED_WHALE_LIGHT_THEME
            }
        }
        ConnectionStatus::Connecting => STARTING_COLOR,
        ConnectionStatus::PermissionDenied => PERMISSION_DENIED_COLOR,
        ConnectionStatus::Disconnected => PROBLEM_COLOR,
    }
}

fn tray_service_state(snapshot: &ApplicationSnapshot) -> TrayServiceState {
    if snapshot.global_stop_in_progress {
        return TrayServiceState::GlobalStop;
    }

    let active_projects: Vec<_> = snapshot
        .projects
        .iter()
        .filter(|project| project.active)
        .collect();
    if active_projects.is_empty() {
        return TrayServiceState::Neutral;
    }

    let included_services: Vec<_> = active_projects
        .into_iter()
        .flat_map(|project| project.services.iter())
        .filter(|service| service.included)
        .collect();
    if included_services.is_empty() {
        return TrayServiceState::Neutral;
    }

    if included_services.iter().any(|service| {
        matches!(
            service.status,
            ServiceStatus::NotCreated
                | ServiceStatus::Stopped
                | ServiceStatus::Unhealthy
                | ServiceStatus::Error
        )
    }) {
        return TrayServiceState::Problem;
    }

    if included_services.iter().any(|service| {
        matches!(
            service.status,
            ServiceStatus::Starting | ServiceStatus::Paused
        )
    }) {
        return TrayServiceState::Starting;
    }

    TrayServiceState::Ready
}

fn is_status_dot_pixel(pixel: &[u8; 4]) -> bool {
    u16::from(pixel[1]) > u16::from(pixel[0]) + 30 && u16::from(pixel[1]) > u16::from(pixel[2]) + 30
}

fn hidden_whale_body_contains(x: i32, y: i32) -> bool {
    if !(34..=59).contains(&x) || y < 31 {
        return false;
    }

    // The source badge overlaps the whale, so non-connected states restore the obscured belly.
    let horizontal_offset = x - 35;
    let body_bottom = 55 - horizontal_offset * horizontal_offset / 55;
    y <= body_bottom
}

fn service_color(state: TrayServiceState, x: i32, y: i32) -> [u8; 3] {
    match state {
        TrayServiceState::Neutral => NEUTRAL_COLOR,
        TrayServiceState::Ready => READY_COLOR,
        TrayServiceState::Starting => STARTING_COLOR,
        TrayServiceState::Problem => PROBLEM_COLOR,
        TrayServiceState::GlobalStop => {
            let horizontal_offset = x - STATUS_DOT_CENTER.0;
            let vertical_offset = y - STATUS_DOT_CENTER.1;
            if horizontal_offset * horizontal_offset + vertical_offset * vertical_offset
                <= GLOBAL_STOP_CENTER_RADIUS_SQUARED
            {
                GLOBAL_STOP_CENTER_COLOR
            } else {
                PROBLEM_COLOR
            }
        }
    }
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
        ServiceAction::Start => (
            &START_ICON,
            [READY_COLOR[0], READY_COLOR[1], READY_COLOR[2], 255],
        ),
        ServiceAction::Resume => (&START_ICON, [45, 145, 248, 255]),
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
    use super::{
        CONNECTED_WHALE_DARK_THEME, CONNECTED_WHALE_LIGHT_THEME, GLOBAL_STOP_CENTER_COLOR,
        NEUTRAL_COLOR, PERMISSION_DENIED_COLOR, PROBLEM_COLOR, READY_COLOR, STARTING_COLOR,
        TrayThemeState, action_icon, resolve_tray_theme, service_action, tray_icon,
        visible_projects,
    };
    use crate::runtime::{
        ConnectionStatus, RuntimeSupervisor, ServiceAction, ServiceStatus, TestRuntimeSupervisor,
        ThemePreference,
    };
    use tauri::{Theme, image::Image};

    #[test]
    fn synchronized_theme_wins_when_the_window_reports_a_stale_startup_theme() {
        let theme_state = TrayThemeState::default();
        theme_state.synchronize(Theme::Light);

        assert_eq!(theme_state.resolve(Some(Theme::Dark)), Theme::Light);
    }

    #[test]
    fn explicit_light_theme_always_uses_the_light_tray_variant() {
        assert_eq!(
            resolve_tray_theme(ThemePreference::Light, Theme::Dark),
            Theme::Light
        );
    }

    #[test]
    fn explicit_dark_theme_always_uses_the_dark_tray_variant() {
        assert_eq!(
            resolve_tray_theme(ThemePreference::Dark, Theme::Light),
            Theme::Dark
        );
    }

    #[test]
    fn system_theme_uses_the_synchronized_system_variant() {
        assert_eq!(
            resolve_tray_theme(ThemePreference::System, Theme::Dark),
            Theme::Dark
        );
        assert_eq!(
            resolve_tray_theme(ThemePreference::System, Theme::Light),
            Theme::Light
        );
    }

    #[test]
    fn connected_whale_uses_a_dark_light_theme_color_and_a_light_dark_theme_color() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");
        set_active_service_statuses(&mut snapshot, ServiceStatus::Healthy);
        let light = tray_icon(&snapshot, Theme::Light);
        let dark = tray_icon(&snapshot, Theme::Dark);

        assert_eq!((light.width(), light.height()), (64, 64));
        assert_eq!((dark.width(), dark.height()), (64, 64));
        assert_eq!(rgb_at(&light, 20, 35), CONNECTED_WHALE_LIGHT_THEME);
        assert_eq!(rgb_at(&dark, 20, 35), CONNECTED_WHALE_DARK_THEME);
        assert_eq!(rgb_at(&light, 48, 48), READY_COLOR);
        assert_eq!(rgb_at(&dark, 48, 48), READY_COLOR);
    }

    #[test]
    fn connection_states_override_the_whale_color_and_hide_the_status_dot() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");

        for (connection, expected_color) in [
            (ConnectionStatus::Connecting, STARTING_COLOR),
            (ConnectionStatus::PermissionDenied, PERMISSION_DENIED_COLOR),
            (ConnectionStatus::Disconnected, PROBLEM_COLOR),
        ] {
            snapshot.connection = connection;
            for theme in [Theme::Light, Theme::Dark] {
                let icon = tray_icon(&snapshot, theme);
                assert_eq!(rgb_at(&icon, 20, 35), expected_color);
                assert_eq!(alpha_at(&icon, 58, 52), 0);
            }
        }
    }

    #[test]
    fn connected_status_dot_represents_the_active_project_services() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");

        for project in &mut snapshot.projects {
            project.active = false;
        }
        assert_eq!(
            rgb_at(&tray_icon(&snapshot, Theme::Light), 48, 48),
            NEUTRAL_COLOR
        );

        snapshot.projects[0].active = true;
        set_active_service_statuses(&mut snapshot, ServiceStatus::Healthy);
        assert_eq!(
            rgb_at(&tray_icon(&snapshot, Theme::Light), 48, 48),
            READY_COLOR
        );

        snapshot.projects[0].services[0].status = ServiceStatus::Starting;
        assert_eq!(
            rgb_at(&tray_icon(&snapshot, Theme::Light), 48, 48),
            STARTING_COLOR
        );

        snapshot.projects[0].services[1].status = ServiceStatus::Stopped;
        assert_eq!(
            rgb_at(&tray_icon(&snapshot, Theme::Light), 48, 48),
            PROBLEM_COLOR
        );
    }

    #[test]
    fn global_stop_uses_a_red_ring_with_a_white_center() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");
        snapshot.global_stop_in_progress = true;

        for theme in [Theme::Light, Theme::Dark] {
            let icon = tray_icon(&snapshot, theme);
            assert_eq!(rgb_at(&icon, 48, 48), GLOBAL_STOP_CENTER_COLOR);
            assert_eq!(rgb_at(&icon, 48, 39), PROBLEM_COLOR);
        }
    }

    #[test]
    fn connection_state_has_priority_over_global_stop() {
        let supervisor = TestRuntimeSupervisor::new();
        let mut snapshot = run(supervisor.snapshot()).expect("fixture snapshot should load");
        snapshot.connection = ConnectionStatus::Disconnected;
        snapshot.global_stop_in_progress = true;

        let icon = tray_icon(&snapshot, Theme::Light);
        assert_eq!(rgb_at(&icon, 20, 35), PROBLEM_COLOR);
        assert_eq!(alpha_at(&icon, 58, 52), 0);
    }

    fn set_active_service_statuses(
        snapshot: &mut crate::runtime::ApplicationSnapshot,
        status: ServiceStatus,
    ) {
        for project in &mut snapshot.projects {
            if project.active {
                for service in &mut project.services {
                    if service.included {
                        service.status = status;
                    }
                }
            }
        }
    }

    fn rgb_at(image: &Image<'_>, x: usize, y: usize) -> [u8; 3] {
        let offset =
            (y * usize::try_from(image.width()).expect("image width should fit usize") + x) * 4;
        image.rgba()[offset..offset + 3]
            .try_into()
            .expect("pixel should contain three color channels")
    }

    fn alpha_at(image: &Image<'_>, x: usize, y: usize) -> u8 {
        let offset =
            (y * usize::try_from(image.width()).expect("image width should fit usize") + x) * 4;
        image.rgba()[offset + 3]
    }

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
    fn start_action_icon_uses_the_ready_green() {
        let icon = action_icon(ServiceAction::Start);

        assert_eq!(rgb_at(&icon, 5, 2), READY_COLOR);
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
