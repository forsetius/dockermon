# Dockermon

Dockermon is a focused desktop monitor for local Docker development projects on Linux,
especially Zorin OS.

The desktop application monitors and controls the local Docker Engine through a production
runtime. The browser build continues to use the deterministic test runtime so the complete
interface can be developed without changing local Docker resources.

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

## License

Dockermon is source-available software licensed under the Apache License 2.0 subject to the
Commons Clause License Condition v1.0. You may use and modify Dockermon, including for internal
business purposes, but you may not sell the software as defined by the Commons Clause. See
[`LICENSE`](LICENSE) for the complete terms and [`NOTICE`](NOTICE) for attribution information.

Third-party components remain available under their respective licenses. Their notices are stored
in [`THIRD_PARTY_LICENSES`](THIRD_PARTY_LICENSES).
