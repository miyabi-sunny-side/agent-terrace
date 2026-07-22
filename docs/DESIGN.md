---
version: alpha
name: Sumi / agent-terrace
description: >
  agent-terrace project overrides for the Sumi design system. The canonical
  template lives at ~/.dotfiles/agent/common/designs/sumi/DESIGN.md; this file
  records only the terrace accent, agent-state colors, and domain components.
colors:
  accent: "#9a3f16"
  accent-subtle: "rgba(154, 63, 22, 0.12)"
  accent-dark: "#f19a69"
  accent-subtle-dark: "rgba(241, 154, 105, 0.15)"
  secondary: "#155c52"
  secondary-subtle: "rgba(21, 92, 82, 0.12)"
  secondary-dark: "#72cfbd"
  secondary-subtle-dark: "rgba(114, 207, 189, 0.14)"
  state-busy: "#805400"
  state-busy-dark: "#e0b052"
  terminal-surface: "#f4f1ea"
  terminal-surface-dark: "#111312"
---

# agent-terrace — Sumi Project Overrides

## Overview

agent-terrace follows the Sumi design system. Its compact interface keeps the
agent registry and terminal content ahead of application branding. The warm
persimmon accent marks the selected agent and primary navigation without
decorating the terminal content. The secondary teal has one role only: a
registered agent that is idle and ready.

Sumi is the default dark theme. Washi follows the operating-system light-theme
preference and prioritizes e-paper contrast. The app has no theme toggle.

## Domain colors

- **Agent idle:** secondary teal. This means registered and ready; it is used on
  the status marker and status label only.
- **Agent busy:** dark ochre. This means the agent is processing a turn; shape
  and the `busy` label carry the same information without relying on hue.
- **Terminal surface:** a dedicated near-black / warm-paper reading surface.
  ANSI output colors are terminal data, not application chrome.

## Domain components

- **Agent row:** A compact two-line summary led by the tmux location and state.
  Runtime name, pane ID, and abbreviated working directory form the secondary
  line, preserving the relationship between a session and its agent. The
  complete path remains available as a title. The selected row has a 3px accent
  edge; status always includes text and a small square marker.
- **Detail tabs:** Screen and Letters are peer views under the selected agent.
  The active tab is reflected in the URL hash. Screen polling stops while
  Letters is active, and Letters polling stops when its view is unmounted.
- **Screen viewer:** Read-only captured terminal output in a monospace reading
  surface. It scrolls normally on mobile, preserves whitespace, and labels the
  refresh cadence. It never exposes terminal input controls.
- **Letter shelf:** A chronological stack of outgoing instructions and incoming
  replies. Direction, timestamp, skill, and event ID remain visible without
  making the timeline resemble a chat application. Body text preserves
  whitespace and is always rendered as plain text. Letters is a timeline-only
  view; it does not own the composer.
- **Letter dock:** A shared footer under both Screen and Letters. Its collapsed
  state shows only the `手紙` launcher at the lower right. Activating the
  launcher reveals the composer upward without changing the active view, so
  terminal output can remain visible while an instruction is drafted. Closing
  the dock or switching Screen / Letters preserves the draft and selected
  skill; selecting a different agent resets both. Skill is a separate menu,
  not a command prefix in the textarea. Byte count, disabled sending state,
  delivery result, and errors remain close to the submit action; a failed
  delivery never clears the body.
- **Application shell:** There is no global wordmark, registry heading, section
  numbering, or manual refresh control. The registry begins directly with the
  agent rows, while the selected-agent header uses one compact line for the
  back control, runtime name, pane ID, and refresh status.

## Constraints

- Keep terminal output visually dominant once an agent is selected.
- Never use agent state colors for buttons, links, or decoration.
- Keep the Screen view strictly read-only. The only keyboard affordance is the
  structured letter dock shared by both detail views; never add terminal input
  or `send-keys` controls.
- The launcher exposes expansion state and the dock panel relationship to
  assistive technology. The close control and Escape collapse the dock and
  return focus to the launcher. Skill selection supports keyboard navigation.
- Motion is limited to the short upward dock reveal, its chevron, and loading
  indicators. The dock transitions are disabled when reduced motion is
  requested.
