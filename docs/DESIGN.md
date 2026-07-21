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

agent-terrace follows the Sumi design system. Its interface is a quiet lookout:
the warm persimmon accent marks the selected agent and primary navigation,
recalling late sunlight on a terrace without decorating the terminal content.
The secondary teal has one role only: a registered agent that is idle and ready.

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

- **Agent row:** Name, state, tmux location, and abbreviated working directory.
  The complete path remains available as a title. The selected row has a 3px
  accent edge; status always includes text and a small square marker.
- **Detail tabs:** Screen and Letters are peer views under the selected agent.
  The active tab is reflected in the URL hash. Screen polling stops while
  Letters is active, and Letters polling stops when its view is unmounted.
- **Screen viewer:** Read-only captured terminal output in a monospace reading
  surface. It scrolls normally on mobile, preserves whitespace, and labels the
  refresh cadence. It never exposes terminal input controls.
- **Letter shelf:** A chronological stack of outgoing instructions and incoming
  replies. Direction, timestamp, skill, and event ID remain visible without
  making the timeline resemble a chat application. Body text preserves
  whitespace and is always rendered as plain text.
- **Letter composer:** A sticky form at the bottom of Letters only. Skill is a
  separate menu, not a command prefix in the textarea. Byte count, disabled
  sending state, delivery result, and errors remain close to the submit action;
  a failed delivery never clears the body.
- **Lookout header:** A compact sticky bar with the terrace wordmark and a live
  registry count. It contains no decorative iconography.

## Constraints

- Keep terminal output visually dominant once an agent is selected.
- Never use agent state colors for buttons, links, or decoration.
- Keep the Screen view strictly read-only. The only keyboard affordance is the
  structured composer inside Letters; never add terminal input or `send-keys`
  controls.
- Motion is limited to a short opacity reveal and the loading indicator; Washi
  disables the reveal under reduced-motion and light-theme preferences.
