# Dockermon

Dockermon is a focused desktop monitor for local Docker development projects on Zorin OS.

The project is being delivered in acceptance-gated stages. Stage 1 provides the complete application
interface and native tray behavior through a deterministic test runtime. It does not connect to Docker
yet.

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
