# Dockermon

Dockermon is a focused desktop monitor for local Docker development projects on Linux,
especially Zorin OS.

See the [user guide](USER_GUIDE.md) for installation, first-run setup, everyday workflows,
tray behavior, and troubleshooting.

The desktop application monitors and controls the local Docker Engine through a production
runtime. The browser build continues to use the deterministic test runtime so the complete
interface can be developed without changing local Docker resources.

## Installation

Install Dockermon from its signed APT repository to receive future updates:

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSLo /tmp/dockermon-archive-keyring.gpg \
  https://apt.forseti.pl/dockermon-archive-keyring.gpg
sudo install -m 0644 /tmp/dockermon-archive-keyring.gpg \
  /etc/apt/keyrings/dockermon-archive-keyring.gpg
curl -fsSLo /tmp/dockermon.sources https://apt.forseti.pl/dockermon.sources
sudo install -m 0644 /tmp/dockermon.sources \
  /etc/apt/sources.list.d/dockermon.sources
sudo apt update
sudo apt install dockermon
```

Alternatively, download the Debian package for the current release and install it directly:

```bash
sudo apt install ./Dockermon_0.1.4_amd64.deb
```

Dockermon requires a running Docker Engine, the Docker Compose plugin, and permission to access
the local Docker socket. See the [user guide](USER_GUIDE.md#requirements) for setup and
verification steps.

Repository setup, signing, publishing, and validation are documented in
[`docs/APT_REPOSITORY.md`](docs/APT_REPOSITORY.md).

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

Build the Debian package and normalize its desktop integration files with:

```bash
pnpm bundle:deb
```

The package is written to `src-tauri/target/release/bundle/deb/`.

Run the destructive Docker integration scenarios only through the isolated smoke-test runner:

```bash
pnpm test:docker-smoke
```

The runner starts a disposable Docker-in-Docker Engine, points both Bollard and Docker Compose at
its private Unix socket, and stores Dockermon configuration in a temporary XDG directory. The
fixtures use only names prefixed with `dockermon-smoke`; the test never stops or removes resources
from the user's Docker Engine.

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
