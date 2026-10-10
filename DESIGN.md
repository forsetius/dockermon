---
name: Dockermon
description: A calm, state-first local Docker operations console.
colors:
  ink: '#152033'
  accent: '#0568d8'
  accent-hover: '#0059bb'
  accent-soft: '#e5f1ff'
  accent-text: '#046ccc'
  app-background: '#f4f7fb'
  border: '#dbe3ed'
  border-strong: '#c9d4e2'
  control-border: '#bdcad9'
  control-muted: '#d7dfe9'
  danger: '#cf2330'
  danger-soft: '#fff0f1'
  focus-ring: 'rgb(8 127 245 / 34%)'
  header-surface: '#fbfcfe'
  muted: '#647187'
  muted-soft: '#647187'
  panel: '#ffffff'
  panel-hover: '#f7faff'
  panel-selected: '#e7f2ff'
  sidebar: '#f8fafc'
  success: '#087f3a'
  warning: '#925600'
  terminal: '#101821'
  terminal-text: '#dce6ef'
  log-method-get: '#63d995'
  log-method-post: '#68aefc'
  log-method-patch: '#4bd2c5'
  log-method-put: '#c59bf5'
  log-method-delete: '#ff7784'
  log-method-head: '#ffb45f'
  log-status-success: '#65d99a'
  log-status-redirect: '#f1d365'
  log-status-client-error: '#ffad55'
  log-status-server-error: '#ff6875'
  log-time: '#f8fbff'
  log-level-info: '#f8fbff'
  log-level-warning: '#f1d365'
  log-level-error: '#ffad55'
  log-level-fatal: '#ff6875'
  on-accent: '#ffffff'
typography:
  headline:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '1.55rem'
    fontWeight: 760
    lineHeight: 1.2
    letterSpacing: '-0.025em'
  title:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.98rem'
    fontWeight: 630
  body:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.82rem'
    fontWeight: 400
  label:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.75rem'
    fontWeight: 650
  button:
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    fontSize: '0.78rem'
    fontWeight: 650
  mono:
    fontFamily: "'DejaVu Sans Mono', ui-monospace, monospace"
    fontSize: '0.76rem'
    fontWeight: 400
    lineHeight: 1.62
  log:
    fontFamily: "'JetBrains Mono Variable', 'JetBrains Mono', 'DejaVu Sans Mono', monospace"
    fontSize: '0.76rem'
    fontWeight: 400
    lineHeight: 1.62
rounded:
  checkbox: '4px'
  compact: '7px'
  control: '8px'
  navigation: '9px'
  panel: '12px'
  state-icon: '14px'
  pill: '999px'
spacing:
  micro: '4px'
  compact: '8px'
  regular: '12px'
  comfortable: '16px'
  content: '24px'
components:
  button-primary:
    backgroundColor: '{colors.accent}'
    textColor: '{colors.on-accent}'
    typography: '{typography.button}'
    rounded: '{rounded.control}'
    padding: '0 13px'
    height: '36px'
  button-primary-hover:
    backgroundColor: '{colors.accent-hover}'
    textColor: '{colors.on-accent}'
    typography: '{typography.button}'
    rounded: '{rounded.control}'
    padding: '0 13px'
    height: '36px'
  button-danger:
    backgroundColor: '{colors.danger-soft}'
    textColor: '{colors.danger}'
    typography: '{typography.button}'
    rounded: '{rounded.control}'
    padding: '0 13px'
    height: '36px'
  button-icon:
    backgroundColor: '{colors.panel}'
    textColor: '{colors.muted}'
    rounded: '{rounded.compact}'
    size: '32px'
  navigation-active:
    backgroundColor: '{colors.accent-soft}'
    textColor: '{colors.accent-text}'
    rounded: '{rounded.navigation}'
    height: '44px'
  project-panel:
    backgroundColor: '{colors.panel}'
    textColor: '{colors.ink}'
    rounded: '{rounded.panel}'
  toggle-on:
    backgroundColor: '{colors.accent}'
    rounded: '{rounded.pill}'
    size: '42px 24px'
  checkbox-bulk:
    backgroundColor: '{colors.panel}'
    textColor: '{colors.on-accent}'
    rounded: '{rounded.checkbox}'
    size: '18px'
  checkbox-bulk-selected:
    backgroundColor: '{colors.accent}'
    textColor: '{colors.on-accent}'
    rounded: '{rounded.checkbox}'
    size: '18px'
  select-field:
    backgroundColor: '{colors.panel}'
    textColor: '{colors.ink}'
    rounded: '{rounded.control}'
    padding: '0 36px 0 12px'
    height: '40px'
---

# Design System: Dockermon

## Overview

**Creative North Star: "The Local Control Room"**

Dockermon is a calm, compact operations console for a developer's own machine. The interface leads with operational state, keeps routine actions close to the affected service, and uses restrained blue emphasis so running, warning, and failure colors remain immediately meaningful.

Its visual character is native-feeling rather than branded spectacle: crisp system typography, pale tonal separation, compact controls, fine borders, and a persistent workspace frame. Light and dark themes preserve the same information hierarchy. The result should feel at home on a Linux desktop without reproducing Docker Desktop's visual identity.

**Key Characteristics:**

- Dense, scan-first operational rows with stable columns and tabular metrics.
- A single blue interaction accent, reserved semantic state colors, and quiet neutral surfaces.
- Flat panels separated by borders; elevation appears only for temporary overlays.
- Desktop sidebar navigation that becomes a labeled bottom bar on narrow screens.
- Monospaced ports and logs inside an otherwise system-sans interface.

## Colors

The palette combines a cool blue operational accent with blue-grey neutrals and unambiguous green, amber, and red state signals.

### Primary

- **Operational Blue:** Drives primary actions, selected navigation, enabled toggles, checkboxes, and focus-adjacent emphasis.
- **Selection Wash:** Marks active navigation and selected service rows without overpowering status colors.

### Secondary

- **Healthy Green:** Means connected, running, healthy, or successful only.
- **Transition Amber:** Means connecting, starting, or paused only.
- **Failure Red:** Means stopped, disconnected, unhealthy, error, or an explicit stop action.

### Neutral

- **Cool Canvas:** The application background keeps white panels legible without introducing card shadows.
- **Panel White:** Contains projects, settings, activity, and toast content.
- **Blue-grey Borders:** Separate rows, surfaces, controls, and window regions with two strengths.
- **Deep Ink and Muted Slate:** Establish the content hierarchy while keeping metadata and metrics quiet.
- **Terminal Navy:** Gives logs a dedicated high-contrast working surface with pale monospaced text.

The terminal surface uses a restrained syntax palette for scan speed: GET and 2xx are green, POST
is blue, PATCH is teal, PUT is purple, DELETE and 5xx are red, HEAD and 4xx are orange, 3xx is
yellow, and the time portion of timestamps is bright white. Dark mode raises those syntax colors
slightly (`#6ee7a7`, `#78b8ff`, `#5ce1d3`, `#d0a7ff`, `#ff8793`, `#ffc171`, `#72e3ab`,
`#ffe078`, `#ffbc68`, `#ff7b87`, and `#ffffff`, respectively) against the darker terminal surface.
Labels, numeric ranges, and fixed token positions preserve meaning without relying on color alone.
Log levels reuse the same semantic ramp: LOG and INFO are bright white, WARN and WARNING are
yellow, ERROR and ERR are orange, and FATAL is red. The drawer toolbar places the Coloring toggle
directly after Follow. Coloring is enabled by default and its session-local state applies to every
service until the application exits.

Dark mode keeps these semantic roles and overrides their values as follows: accent (`#2d91f8`), accent hover (`#4ca1f8`), accent wash (`#15395d`), accent text (`#69b2fb`), canvas (`#0d151d`), border (`#2b3744`), strong border (`#3a4857`), control border (`#516172`), muted control (`#34414e`), danger (`#ff525e`), danger wash (`#3b1c22`), header (`#121b24`), muted text (`#a8b4c2`), quiet text (`#7f8c9b`), panel (`#141e27`), hover panel (`#192631`), selected panel (`#193b5e`), sidebar (`#101922`), success (`#2bdc68`), warning (`#ffb31a`), and terminal (`#080d12`). Primary text becomes `#eef4fa`; terminal text remains unchanged.

**The State Color Rule.** Green, amber, and red communicate runtime state or operation consequence; never use them as decorative accents.

**The One Accent Rule.** Blue owns interaction and selection. New screens should not introduce a competing brand accent.

## Typography

**Display Font:** Inter (with the system sans-serif stack)
**Body Font:** Inter (with the system sans-serif stack)
**Data Mono Font:** DejaVu Sans Mono (with the system monospace stack)
**Log Font:** Bundled JetBrains Mono Variable, with ligatures disabled and system monospace fallbacks

**Character:** The sans-serif hierarchy is compact, direct, and optimized for a desktop utility. Weight and size create hierarchy without decorative type. DejaVu Sans Mono remains the compact data face for ports, while the bundled JetBrains Mono improves small-text legibility in the log stream. Log ligatures are disabled so operators and punctuation preserve their literal form.

### Hierarchy

- **Headline** (760, 1.55rem, 1.2): Page titles; compact and visually decisive.
- **Title** (630, 0.98rem): Project names and other local container titles.
- **Body** (400, 0.82rem): Service rows, metrics, activity metadata, and supporting interface text.
- **Label** (650, 0.75rem): Table headers and compact metadata labels; mobile field labels become uppercase with modest tracking.
- **Button** (650, 0.78rem): Compact action labels paired with small outline icons.
- **Mono** (400, 0.76rem, 1.62): Ports and the log viewport.

**The Monospace Evidence Rule.** Use monospace only for machine output, port mappings, and similarly inspectable values—not for navigation or prose.

## Layout

The desktop shell is a fixed viewport grid with a left sidebar (210px), a top window header (62px), and a scrollable content region. The main project view uses the full available content width with 24px outer padding so operational columns benefit from large desktop windows; secondary and settings views remain constrained to 960px for comfortable reading. Project panels stack with 18px gaps; rows remain compact at 40–44px high so several services stay visible at once.

Below 980px, the sidebar collapses to a 74px icon rail. The same rail can be enabled manually at wider window sizes through the sidebar control in the window header; the control remains next to the sidebar boundary as the layout changes. As the table loses horizontal space, observability columns disappear in a fixed priority order: ports first, then memory, then CPU. The rule responds to both window width and the table's actual container width, including space lost to the log drawer. When the projects workspace is narrower than 900px, bulk start and stop actions retain their localized accessible names but display only their icons so the project header remains on one row. Below 700px, the sidebar becomes a fixed 64px bottom navigation bar, the top header becomes 56px, and the content padding becomes 18px 14px 88px. Service rows become compact cards that retain the bulk checkbox, service name, state, and actions. Project actions form a two-column grid, with the active-project toggle on its own row and full bulk-action labels restored.

The log drawer adapts to the usable workspace width measured before the drawer opens, preventing layout feedback loops. At 1360px or more it docks on the right, is 560px wide by default, and can be resized from 360px up to 1100px while reserving at least 600px for the workspace whenever possible. Below that threshold it docks at the bottom, is 380px high by default, and can be resized from 240px up to 720px while reserving at least 280px of workspace height. The workspace yields the drawer's current width or height instead of being obscured. The adjacent edge is pointer-draggable and keyboard-operable; double-clicking restores the default size. At 700px and below, the drawer uses the full viewport, hides the resize separator, and retains its modal backdrop behavior.

The native tray keeps one submenu per visible project and exactly one item per service. Each item is labeled with the service name and uses its icon and click behavior for the single recommended action in the current state: start when absent, stopped, or failed; stop while starting, running, or healthy; restart when unhealthy; and resume when paused. When lifecycle operations are unavailable globally, service items remain visible but disabled.

The projects heading pairs the active-project count with two compact import actions for a Compose directory or an ordered file selection. Optional Compose profiles appear beside the project identity as small checkbox controls. Services belonging only to disabled profiles remain in the table with a neutral `Profile disabled` badge while retaining their actual runtime state, but their bulk selection and lifecycle controls are unavailable until a profile includes them. A compact overflow menu at the end of each project toolbar contains secondary project actions. Its remove action affects only Dockermon's catalog entry and preferences; the localized success message explicitly confirms that Compose files and Docker resources were not changed.

**The Scan Before Action Rule.** Keep names, state, CPU, memory, ports, and row actions in a consistent reading order. When horizontal space is insufficient, hide ports, then memory, then CPU; never hide the service name, state, or actions.

## Elevation & Depth

The system is flat by default. Borders, surface tone, and selected-row fills establish hierarchy; persistent cards and navigation do not cast shadows. Only temporary overlays rise above the workspace.

### Shadow Vocabulary

- **Floating Feedback** (`0 18px 42px rgb(30 49 76 / 15%)`; dark: `0 18px 42px rgb(0 0 0 / 28%)`): Toast notifications only.
- **Drawer Separation** (`-14px 0 36px rgb(23 37 58 / 16%)` on the right or `0 -14px 36px rgb(23 37 58 / 16%)` at the bottom; dark mode increases opacity): The log drawer edge.
- **Toggle Thumb** (`0 2px 5px rgb(0 0 0 / 22%)`): The movable white switch thumb.

**The Flat Workspace Rule.** Resting workspace surfaces use borders and tonal contrast; shadows are reserved for transient layers or a movable control part.

## Shapes

Dockermon uses gently rounded rectangles rather than pills for structural UI. Project and settings panels use a 12px radius; standard controls use 8px; compact icon actions use 7px; navigation items use 9px; bulk checkboxes use 4px. State icons use 14px containers. Fully rounded geometry is limited to status dots, the toggle track, and scrollbar thumbs.

Borders are one pixel and cool-toned. Panels clip their internal row separators to the outer radius. Icons are compact, unfilled line drawings with rounded joins; action glyphs may use solid play or stop shapes for faster recognition.

**The Radius Hierarchy Rule.** Containers are rounder than their controls, and controls are rounder than dense row geometry.

## Components

### Buttons

- **Shape:** Compact rectangular controls with gently curved corners (8px), a minimum height of 36px, and 13px horizontal padding.
- **Primary:** Solid operational blue with white text and an action icon.
- **Hover / Focus:** Darker blue on hover; every keyboard focus receives a visible 3px translucent blue outline with a 2px offset.
- **Danger:** Pale red surface, red text, and a mixed red border; hover strengthens the border without filling the button solid red.
- **Icon Action:** A 32px square, white or themed panel surface, strong neutral border, and 7px radius. Hover shifts border, glyph, and background into the accent family.
- **Text Action:** Transparent and muted by default, becoming primary text on hover.

### Chips

- **Style:** Count badges are compact neutral chips with a one-pixel border, panel background, 8px corners, and semibold muted text.
- **State:** They summarize information only; selection is expressed through rows, navigation, checkboxes, or toggles instead.

### Cards / Containers

- **Corner Style:** Gently rounded outer panels (12px).
- **Background:** The semantic panel surface over the cooler application canvas.
- **Shadow Strategy:** Flat at rest; see Elevation & Depth.
- **Border:** One-pixel neutral border with row separators inside.
- **Internal Padding:** Headers use compact asymmetric padding; data rows carry their own cell padding rather than a large card inset.

### Inputs / Fields

- **Style:** Select fields use a strong neutral stroke, panel background, 8px radius, and a 40px minimum height. Bulk checkboxes are fixed 18px squares in every state, with a crisp white check or horizontal mixed-state bar on the blue accent.
- **Focus:** The global 3px focus outline applies without removing native semantics.
- **Disabled:** Controls reduce to 48% opacity and retain their shape; the cursor signals unavailability.

### Navigation

The desktop sidebar uses 44px rows with 9px corners, muted labels, and a blue-wash active state. Settings is anchored at the bottom behind a full-width separator, while the operational destinations stay grouped below the brand. A localized, keyboard-accessible control in the window header toggles the sidebar between its full width and icon rail on desktop. Responsive collapse still takes precedence below 980px, and the sidebar then becomes a persistent four-item bottom bar with both icons and labels below 700px. Hover uses only a quiet panel tint; selection uses the accent family and `aria-current`.

### Project Panel

Each project is an expandable, bordered container with its identity on the left and project-scope controls on the right. The active-project control reserves a fixed 124px slot so changing between active and inactive labels never moves the switch. Service rows keep selection, identity, state, metrics, ports, and actions in stable columns; CPU and memory values use tabular numerals aligned to the right. The mobile version converts the same information to labeled row cards without losing actions or metrics.

Lifecycle operations disable only the affected project, so independent projects remain actionable.
During the Engine-wide stop operation, every lifecycle control is disabled and a compact semantic
status strip reports preparation or completed/total progress. Its red treatment communicates the
consequence of the operation, not a failure.

### Toggle

The toggle is a 42px by 24px pill with a 16px white thumb. Its off state is neutral and bordered; its on state uses the operational blue. The thumb moves 18px with a 180ms emphasized ease, and the hidden checkbox retains focus and accessibility semantics.

### Log Drawer

The drawer is the primary inspection surface: a panel header and toolbar above a full-height terminal navy viewport. Logs use pale monospaced text, preserve whitespace, and expose follow, clear, close, Escape, focus restoration, and mobile backdrop behavior. Opening the drawer follows the selected service by default; changing services replaces the previous stream, clearing affects only the visible buffer, and an empty stream shows a quiet waiting message. The viewport retains a bounded history so long-running local services do not grow the webview indefinitely.

### Loading, Empty, Error, and Toast States

Loading uses panel-shaped skeletons with a restrained 1.3s sweep. Empty and error states center a 58px semantic icon tile, concise copy, and one primary recovery action. Toasts are compact, dismissible overlays; success and error variants alter the border rather than flooding the surface with color. A partially successful global stop uses the error variant with a concise count first and a bounded, scrollable list of affected container names below it.

## Do's and Don'ts

### Do:

- **Do** lead with current Docker state before presenting lifecycle actions.
- **Do** preserve the stable name → state → CPU → memory → ports → actions reading sequence across breakpoints.
- **Do** pair semantic status color with text or an icon rather than relying on color alone.
- **Do** keep both themes semantically equivalent and use the existing CSS custom-property roles.
- **Do** use restrained 140–200ms transitions and honor reduced-motion preferences.
- **Do** keep routine project actions visibly scoped to the current panel.

### Don't:

- **Don't** use green, amber, or red for decoration or navigation emphasis.
- **Don't** add resting card shadows, gradients, glass effects, or oversized floating surfaces to the workspace.
- **Don't** hide service names, states, or actions when adapting the table, and never hide observability columns out of the ports → memory → CPU priority order.
- **Don't** turn every control into a pill; reserve fully rounded shapes for switches, dots, and scrollbar thumbs.
- **Don't** expand the type palette beyond system sans and functional monospace without replacing the visual system deliberately.
- **Don't** imitate Docker Desktop's visual identity; Dockermon should remain a quiet Linux-native utility.
