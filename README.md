# Dockermon

Dockermon is a focused desktop monitor for local Docker development projects on Zorin OS.

The project is being delivered in acceptance-gated stages. Stage 0 contains the Tauri and Svelte
application shell, system tray integration, single-instance behavior, and the initial quality gates.

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

## Stage 0 acceptance

- The main window opens on Zorin OS 18.1.
- The Dockermon tray icon is visible and opens its native menu.
- Closing the main window hides it without quitting the application.
- Selecting **Open Dockermon** restores and focuses the window.
- Launching a second instance focuses the existing window.
- Selecting **Quit** exits the application.
