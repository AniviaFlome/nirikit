# AGENTS.md

## Build & test

Rust is not installed system-wide. Use the Nix devShell (direnv loads it automatically via `.envrc`, or run `nix develop`):

```sh
cargo fmt --all && cargo test && cargo clippy --all-targets -- -D warnings
nix build --print-build-logs   # release build (runs tests in release mode too)
./result/bin/nirikit --help
```

`gcc` is required as the linker — without it `cargo build` fails with `linker cc not found`. It's included in the devShell.

`nix fmt` formats `.rs` (rustfmt) and `.nix` (nixfmt) via treefmt. `nix flake check` runs the formatting check.

## Architecture

- `src/lib.rs` — library crate re-exporting all modules as `pub` (needed for integration tests in `tests/`).
- `src/main.rs` — thin binary entry point; uses `nirikit::` paths, no `mod` declarations.
- `src/cli.rs` — clap derive structs (`Cli`, `LaunchArgs`, `ProfileArgs`).
- `src/launch.rs` — core launch logic: process spawn, niri event-stream window detection, workspace move, column placement, focus restoration.
- `src/ipc.rs` — raw niri Unix socket protocol (JSON over `UnixStream`).
- `src/profile.rs` — TOML config parsing, `Profile`/`CommandOverrides`/`Position`/`ProfileOverrides` types, merge logic.
- `src/model.rs` — `Workspace`/`Window` structs and `resolve_workspace`.
- `tests/` — integration tests, one file per module. No unit tests inside source files.

## niri IPC quirks

- IPC is JSON over a Unix socket at `$NIRI_SOCKET`, one request per line, one reply per line.
- A socket in `EventStream` mode stops accepting further requests — need a separate connection for actions while streaming.
- `MoveColumnToIndex` is **1-based** (1 = first column).
- `MoveColumnTo*` actions operate on the **focused** column, so placement requires briefly focusing the new window, then restoring focus.
- The event stream must be subscribed **before** spawning the process to avoid missing fast-opening windows.
- `strict-new-window-focus-policy` (niri debug option) prevents heuristic focus on tokenless windows; nirikit removes `XDG_ACTIVATION_TOKEN` when `--no-focus` is set to leverage this.
- niri has no token/workspace-hint mechanism like Hyprland's `HL_INITIAL_WORKSPACE_TOKEN` — windows can only be moved *after* they open, so a brief flicker on the wrong workspace is unavoidable externally.

## Config file

- Location: `--config` flag > `$NIRIKIT_CONFIG` > `$XDG_CONFIG_HOME/nirikit/config.toml` > `dirs::config_dir()/nirikit/config.toml`.
- TOML with `#[serde(rename_all = "kebab-case")]` on structs — fields are `no-focus`, `workspace-id`, `match-app-id`, etc.
- Multi-window profiles use named child tables: `[profiles.dev.01-editor]`, not `[[windows]]` arrays.
- `BTreeMap` for windows means alphabetical key order controls launch order — use `01-`, `02-` prefixes.
- `Position` accepts integers or `"first"`/`"last"`; custom `Deserialize` + `FromStr` impls handle both.

## Flag conventions

- Bool flags use clap's `Set` action with `default_missing_value = "true"` and `num_args = 0..=1` so `--flag` enables and `--flag=false` disables (see `ProfileOverrides::no_focus`, `silent`).
- When adding a bool flag to `LaunchArgs`, also add it to `Profile`, `CommandOverrides` (as `Option<bool>`), and `ProfileOverrides`, then wire through `merge()` and `to_launch_args()`.

## Nix flake

- `nix/package.nix` — Rust package derivation.
- `nix/module.nix` — Home Manager module (install package + generate config via `pkgs.formats.toml {}`). Exposed as `homeModules.default`.
- `nix/treefmt.nix` — treefmt-nix module (rustfmt with edition 2021 + nixfmt).