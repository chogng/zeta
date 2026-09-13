# `ash-terminal-detection`

This crate identifies the terminal program, active multiplexer and color fidelity from process
environment. It also resolves an OSC-reported background or `COLORFGBG` fallback into a light/dark
appearance with an explicit evidence source. It does not read terminal input, send OSC/CSI queries,
emulate a child terminal, manage a PTY, or choose a Ash theme.

| Symbol | Responsibility |
| --- | --- |
| `detect_host_terminal` | Cached process-wide terminal, multiplexer and color-level detection |
| `HostTerminal` | Structured program, version, TERM and multiplexer metadata |
| `TerminalKind` | Stable known-terminal category used by product adapters |
| `ColorLevel` | TrueColor, ANSI-256, ANSI-16 or monochrome fidelity |
| `resolve_background` | OSC 11 RGB → `COLORFGBG` → conservative Dark resolution |

The TUI owns exclusive terminal-response probe windows because those reads must be coordinated with
its crossterm event stream. `ash-terminal` separately owns child-terminal emulation, while
`ash-utils-pty` owns process and PTY plumbing.

```text
ash-tui
├─ ash-terminal-detection  # environment identity
├─ terminal_probe           # OSC query while TUI exclusively owns stdin
└─ features/theme           # TUI-owned palettes, preference, and terminal colors
```

Detection favors `TERM_PROGRAM`, then terminal-specific variables, then `TERM`; tmux and Zellij
program markers identify the multiplexer and do not hide the underlying terminal or overwrite its
version. Ghostty is recognized through its program name, resources directory, or `xterm-ghostty`.
Detection never starts helper processes. TUI startup failures include these same terminal facts
and retain the original I/O error and terminal-mode cleanup.

```bash
just test ash-terminal-detection
bazel test //ash-rs/terminal-detection:terminal-detection-unit-tests
```
