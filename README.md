# rust-pad

Terminal Rust scratchpad — write, compile, and run Rust snippets without leaving the terminal.

## Features

- Inline text editor with syntax-aware highlighting
- F5 to compile and run via `rustc` (10-second timeout, output shown inline)
- F6 to compile-only (check for errors without executing)
- Save/load snippets to `~/.config/rust-pad/snippets/`
- Execution history browser (F4) — recall previous runs
- Starter templates cycled with Tab on empty buffer
- Load a `.rs` file directly from the command line

## Install

```
cargo build --release
# binary at target/release/rust-pad
```

Requires `rustc` in PATH.

## Usage

```
# start with empty scratchpad
rust-pad

# load an existing file
rust-pad --file sketch.rs
```

## Keybindings

| Key | Action |
|-----|--------|
| `F5` | Compile and run |
| `F6` | Compile only (check errors) |
| `F2` | Save snippet |
| `F3` | Load snippet browser |
| `F4` | History browser |
| `Tab` | Cycle template (empty buffer) / insert indent |
| `Ctrl-Q` | Quit |
| `Ctrl-D` | Delete selected item (in browser) |
| `Esc` | Cancel / close dialog |

---
Built with Rust + ratatui
