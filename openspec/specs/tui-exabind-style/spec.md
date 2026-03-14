# tui-exabind-style Specification

## Purpose
TBD - created by archiving change tui-exabind-style. Update Purpose after archive.
## Requirements
### Requirement: Default TUI theme uses Catppuccin (Mocha)
The system MUST define a default TUI theme based on the Catppuccin (Mocha) palette, and MUST map UI semantic roles to that palette.

#### Scenario: Theme roles are available for all widgets
- **WHEN** the TUI starts and constructs its theme
- **THEN** the theme MUST expose semantic roles (background, text, muted, accent, border, selection, search highlight)
- **THEN** those roles MUST resolve to Catppuccin (Mocha) colors

---

### Requirement: Theme supports terminal-default background mode
The system MUST support a mode where the TUI uses the terminal's default background (`bg=Reset`) for the app background, so terminals with window transparency (e.g., Warp) can keep a unified translucent background across the frame while still allowing panels to use explicit theme background colors for readability.

#### Scenario: Terminal-default background mode preserves transparency
- **WHEN** terminal-default background mode is enabled
- **THEN** app background MUST use `bg=Reset`
- **THEN** panel background MUST use the theme's panel background color (e.g., Catppuccin `base`) to keep content readable
- **THEN** border glyphs and foreground colors MUST still use the theme's semantic roles

---

### Requirement: Exabind-style panel border glyph set
The system MUST render framed panels using an exabind-style border glyph set (e.g., `▟▜▔▏▕`) as the default in supported terminals.

#### Scenario: Panel borders use the exabind glyphs
- **WHEN** a standard panel (e.g., Instances) is rendered
- **THEN** its border MUST use the configured exabind-style glyphs rather than the default ASCII/box drawing set

---

### Requirement: Focus and selection styling is theme-driven
The system MUST render focus borders and selection/highlight states using theme-defined styles, and MUST provide clear visual distinction between focused and unfocused panels.

#### Scenario: Focused panel has accented border
- **WHEN** the user focus is on the Instances pane
- **THEN** the Instances pane border MUST use the theme's focused/accented border style
- **THEN** unfocused panes MUST use the theme's muted/default border style

---

### Requirement: Startup open animation
When the TUI enters the alternate screen, it MUST play a staged open animation that reveals the UI in a deterministic order, and MUST complete within a bounded duration.

#### Scenario: Animation runs then yields to steady-state rendering
- **WHEN** the TUI starts with animations enabled
- **THEN** the open animation MUST run for at most 2000ms
- **THEN** the UI MUST reach a steady state where normal rendering continues without animation gating input

#### Scenario: Multi-pane UIs reveal panes sequentially
- **WHEN** the TUI renders a multi-pane layout (e.g., parallel Supervisor TUI)
- **THEN** the open animation MUST reveal major panes sequentially (one completes before the next starts)
- **THEN** the reveal order MUST follow left-to-right, top-to-bottom

#### Scenario: Animation starts from a fully hidden state (no flash)
- **WHEN** the TUI starts with animations enabled
- **THEN** the open animation MUST start from a fully hidden/blank screen state (no UI panes visible)
- **THEN** the UI MUST only become visible as the animation progresses (avoid “render everything, then animate” flicker)

#### Scenario: Terminal-default background mode uses symbol masking
- **WHEN** terminal-default background mode is enabled (`bg=Reset`)
- **THEN** the open animation MUST NOT rely on color interpolation to hide content
- **THEN** the animation MUST use symbol-based masking (e.g., revealing by replacing hidden cells with spaces) to ensure the initial state is truly blank

---

### Requirement: Animations can be disabled or reduced
The system MUST allow disabling or reducing animations for accessibility and deterministic environments, and MUST fall back to immediate static rendering when animations are disabled.

#### Scenario: Reduced motion disables startup animation
- **WHEN** animations are disabled (e.g., by config or environment)
- **THEN** the TUI MUST render the full steady-state UI immediately on first draw

