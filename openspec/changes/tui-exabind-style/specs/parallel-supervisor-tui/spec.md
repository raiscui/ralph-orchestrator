## ADDED Requirements

### Requirement: Supervisor TUI uses the exabind-style theme
In parallel mode, the Supervisor TUI MUST apply the `tui-exabind-style` theme (colors + border glyph set) across header/footer and all major panes (Instances, Output, Chat, Gates).

#### Scenario: All panes share consistent framing and colors
- **WHEN** the Supervisor TUI is running in parallel mode
- **THEN** Instances/Output/Chat/Gates MUST use the same border glyph set and a consistent background strategy
- **THEN** focus border changes MUST follow the theme's focused/unfocused styles

#### Scenario: Adjacent panes are visually separated (no border collapsing)
- **WHEN** the Supervisor TUI is running in parallel mode
- **THEN** the Instances and Output panes MUST NOT have touching borders
- **THEN** there MUST be a visible gap between the two panes while preserving each pane's full border

#### Scenario: Warp preserves terminal-default background
- **WHEN** the Supervisor TUI is running in Warp (e.g., `TERM_PROGRAM` contains `"warp"`)
- **THEN** the TUI MUST use terminal-default background (`bg=Reset`) for the app background to preserve Warp's window transparency
- **THEN** panes MUST still use the theme panel background color for readability (e.g., Catppuccin `base`)
- **THEN** border glyphs and foreground colors MUST still follow the theme

---

### Requirement: Supervisor TUI plays an open animation on startup
When the Supervisor TUI starts (TTY + TUI enabled), it MUST run the `tui-exabind-style` startup open animation once before entering steady-state rendering.

#### Scenario: Startup animation is visible and bounded
- **WHEN** a user starts `ralph run` with `parallel.enabled=true` and TUI enabled
- **THEN** the TUI MUST show a brief open animation
- **THEN** the open animation MUST reveal panes sequentially (Instances → Output → Chat/Gates)
- **THEN** Instances list items MUST start appearing only after the Instances frame animation completes
- **THEN** after the animation completes, all panels MUST be fully rendered and interactive

#### Scenario: Startup begins from a blank screen (no pre-flash)
- **WHEN** a user starts `ralph run` with `parallel.enabled=true`, TUI enabled, and animations enabled
- **THEN** the first rendered frame of the open animation MUST be visually blank (no header/footer/panes visible)
- **THEN** panes MUST only become visible as the staged reveal progresses (never “fully visible first, then animated”)

---

### Requirement: Output pane reopens when switching instances
When the selected instance changes in the Supervisor TUI, the Output pane MUST play a re-open animation that hides the pane and then reveals it again.

#### Scenario: Switching instance triggers output re-open
- **WHEN** the user switches the selected instance in the Instances pane
- **THEN** the Output pane MUST briefly disappear
- **THEN** the Output pane MUST play an open animation to reveal the new output

#### Scenario: Output re-open begins from hidden state (no pre-flash)
- **WHEN** the user switches the selected instance in the Instances pane and animations are enabled
- **THEN** the Output pane MUST NOT render the new output fully visible before the re-open animation starts
- **THEN** the first frame of the re-open animation MUST be visually hidden (no “one-frame flash”)
