# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

## Stack

Delegated and accepted: Tauri 2, Rust, Tokio, Svelte 5, TypeScript, Vite, a Docker Engine adapter based on Bollard, controlled Docker Compose CLI execution, JSON configuration in the XDG application directory, and Debian package distribution.

## Users

The primary user is a local software developer who runs Docker services on a Zorin OS workstation during development.

## Product Purpose

Dockermon makes the state of local Docker development services visible at a glance and provides safe, quick start, stop, and restart actions from both a main window and a taskbar indicator menu. It keeps the projects currently being worked on distinct from other local workloads that are observed mainly for unnecessary resource and port usage.

Success means the user can immediately see whether monitored local services are ready and can perform common lifecycle actions without switching to a terminal.

## Positioning

Dockermon combines automatic discovery of local Docker resources with a deliberately selected, persistent set of monitored development projects and services, surfaced through a Zorin-integrated taskbar workflow.

## Operating Context

The application runs in the background on Zorin OS during local development. It works with local Docker Engine containers, Docker Compose projects and services, and standalone containers. A Compose project may need to remain known after `docker compose down`, when its containers are no longer discoverable from the daemon. The user normally focuses on one project, but may mark multiple projects as active when comparing behavior across them.

## Capabilities and Constraints

- Automatically discover existing local containers and Compose metadata.
- Let the user choose which projects, services, or standalone containers to monitor.
- Persist registered Compose project locations so they can be started when no containers currently exist.
- Let the user mark zero, one, or multiple projects as active work projects.
- Derive the taskbar indicator's aggregate alert state only from active projects so inactive development projects do not produce irrelevant stopped-service alarms.
- Show only active projects in the taskbar menu; show all projects in that menu when no project is active.
- Provide safe start, stop, restart, and related quick actions.
- Scope bulk actions to one project and let service checkboxes define which services participate.
- Show current CPU usage, memory usage, and exposed or published ports for each service.
- Provide a right-side drawer for inspecting the selected service's logs without leaving the project view.
- Provide the tray menu structure: project entries with service counts and submenus, `Stop all`, `Open window`, and `Quit`.
- Make `Stop all` stop every running container visible to the connected local Docker Engine, including Compose and standalone containers, regardless of active-project or bulk selection.
- Support light and dark themes.
- Ship Polish and English localizations with an extensible localization structure for additional languages.
- Keep Docker Swarm outside the initial product scope.
- Treat destructive actions such as removing containers or running `docker compose down` separately from routine quick actions.
- Keep Docker access and process execution in the Rust backend rather than the web frontend.

## Evidence on Hand

No existing interface, visual identity, logo, production screenshots, or user research assets are present yet. Future work must not present illustrative project names or service data as real user data.

## Product Principles

- Make operational state readable before offering actions.
- Preserve the semantic difference between container actions and Compose project actions.
- Keep routine actions fast while isolating destructive operations.
- Keep active-work selection distinct from temporary service selection for bulk actions.
- Keep background resource awareness visible without letting unrelated stopped services dominate the alert state.
- Prefer event-driven updates with reconciliation over blind polling.
- Feel at home on Zorin OS without imitating Docker Desktop.
