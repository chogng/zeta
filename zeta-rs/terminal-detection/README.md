# `zeta-terminal-detection`

This crate identifies the terminal program, active multiplexer and color fidelity from process
environment. It also resolves an OSC-reported background or `COLORFGBG` fallback into a light/dark
appearance with an explicit evidence source. It does not read terminal input, send OSC/CSI queries,
emulate a child terminal, manage a PTY, or choose a Zeta theme.

| Symbol | Responsibility |
| --- | --- |
| `detect_host_terminal` | Cached process-wide terminal, multiplexer and color-level detection |
| `HostTerminal` | Structured program, version, TERM and multiplexer metadata |
| `TerminalKind` | Stable known-terminal category used by product adapters |
| `ColorLevel` | TrueColor, ANSI-256, ANSI-16 or monochrome fidelity |
| `resolve_background` | OSC 11 RGB → `COLORFGBG` → conservative Dark resolution |

The TUI owns exclusive terminal-response probe windows because those reads must be coordinated with
its crossterm event stream. `zeta-terminal` separately owns child-terminal emulation, while
`zeta-utils-pty` owns process and PTY plumbing.

```text
zeta-tui
├─ zeta-terminal-detection  # environment identity
├─ terminal_probe           # OSC query while TUI exclusively owns stdin
└─ zeta-theme               # scheme resolution and token snapshots
```

Detection favors `TERM_PROGRAM`, then terminal-specific variables, then `TERM`; multiplexer identity
is retained independently. It never starts helper processes, so detection is deterministic over the
current process environment.

```bash
cargo test -p zeta-terminal-detection
bazel test //zeta-rs/terminal-detection:terminal-detection-unit-tests
```
