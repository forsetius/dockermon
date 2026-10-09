# Dockermon

Dockermon is a focused desktop monitor for local Docker development projects on Zorin OS.

The project is being delivered in acceptance-gated stages. The desktop application monitors and
controls the local Docker Engine through a production runtime. The browser build continues to use
the deterministic test runtime so the complete interface can be developed without changing local
Docker resources.

## Prerequisites on Zorin OS 18

Install the native build dependencies:

```bash
sudo apt-get install -y \
  pkg-config \
  libdbus-1-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev \
  libxdo-dev
```

Install Rust with rustup, then ensure the user can access the local Docker socket:

```bash
sudo usermod -aG docker "$USER"
```

Log out and back in after changing group membership. Membership in the `docker` group grants
root-equivalent access to the machine.

## Development

```bash
pnpm install
pnpm quality
pnpm tauri dev
```

Run the frontend without the desktop shell with `pnpm dev`.

The browser build uses the same deterministic test data automatically. Append `?scenario=loading`,
`?scenario=empty`, or `?scenario=error` to inspect non-happy-path states.

On Linux, Dockermon selects the KSNI tray backend. Unlike the AppIndicator compatibility backend,
KSNI exports each service action icon through the StatusNotifier menu protocol, so GNOME-based
desktops can render it alongside the service name.

Dockermon stores its versioned configuration in
`$XDG_CONFIG_HOME/dockermon/config.json` or `~/.config/dockermon/config.json`. The file contains
Compose paths, enabled profiles, active projects, bulk selections, language, and theme. Resolved
environment variables and other Compose secrets are neither stored nor sent to the frontend.

## Stage 0 acceptance

- The main window opens on Zorin OS 18.1.
- The Dockermon tray icon is visible and opens its native menu.
- Closing the main window hides it without quitting the application.
- Selecting **Open Dockermon** restores and focuses the window.
- Launching a second instance focuses the existing window.
- Selecting **Quit** exits the application.

## Stage 1 acceptance

- Project and standalone-container views expose simulated status, CPU, memory, ports, and actions.
- Bulk selections affect only their project and remain selected while the application is running.
- Active-project switches control which projects appear in the native tray menu.
- Service actions work from both the main window and the native tray menu.
- The log drawer loads deterministic output, follows simulated live entries, clears only its view,
  and closes with `Escape`.
- System, light, and dark themes and Polish and English interface languages change without restart.
- Loading, empty, and error states are available and the layout adapts to narrow windows.

## Stage 2 acceptance

- The desktop application connects to the system Docker Engine through `/var/run/docker.sock`.
- Compose containers are grouped by their project and service labels; containers without Compose
  labels appear in the **Containers** view.
- Container events update status without a manual refresh, with a full reconciliation every 30
  seconds.
- CPU and memory are sampled every two seconds while the main window is visible. Status monitoring
  continues while the window is hidden.
- Ports and resource usage are aggregated across every container belonging to a Compose service.
- Missing access to the Docker socket and Engine disconnections are shown in the interface without
  terminating the application. Reconnection uses a bounded backoff.
- Logs remain disabled in the production runtime until stage 5. The deterministic browser runtime
  continues to expose them for interface testing.

## Stage 3 acceptance

- Running Compose projects with configuration-path labels are registered automatically and remain
  visible after their containers disappear.
- A project can be imported from a directory or from an ordered selection of Compose files.
- Removing a project forgets its Dockermon catalog entry and preferences without changing Compose
  files or Docker resources. Explicitly removed projects remain ignored until they are imported
  again.
- Dockermon reads every service and profile through `docker compose --profile '*' config --format
json` while retaining only safe catalog metadata.
- Services from disabled profiles remain visible but do not affect project readiness or appear in
  the tray submenu. Enabling a profile includes its services and selects newly available services
  for project bulk actions by default.
- Active projects, profile choices, bulk selections, theme, and language persist across restarts.
- The tray shows active projects, or all projects with a neutral icon when none are active.

## Stage 4 acceptance

- Compose services start with `docker compose up -d`, stop, restart, and resume using the stored
  ordered Compose files, working directory, and enabled profiles. Commands are executed with
  separate arguments and never through a shell.
- Standalone containers are controlled directly through the Docker Engine API.
- Only one lifecycle operation can run in a project at a time. Operations in different projects can
  run concurrently, while the affected project's controls remain disabled.
- Project bulk actions include only services selected in that project and included by its active
  profiles.
- **Stop all** obtains a fresh Engine inventory and stops every running or paused container,
  including inactive and unknown Compose projects and standalone containers. It never removes
  containers, networks, or volumes and never runs `docker compose down`.
- Global stopping uses bounded concurrency, blocks other lifecycle actions, shows progress, and
  reports both the stopped count and individual container failures.
