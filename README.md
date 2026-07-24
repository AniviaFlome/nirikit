# nirikit

Nirikit is a toolkit that adds some missing features to niri.

> [!WARNING]  
> This program is vibecoded. Use at your own risk.

## Features

- Launch applications on specific workspaces without stealing focus.
- Launch applications on the workspace where the command was executed, similar to Hyprland's `initial_workspace_tracking`.

## Usage

```console
nirikit launch -w 3 -n -s -- kitty
nirikit launch --workspace chat --no-focus -- firefox
nirikit launch -w 3 --position first -- kitty
nirikit launch --track-workspace -- kitty
```

Everything after `--` is passed directly to the application.

## Installation

### Home Manager

```nix
{
  imports = [ inputs.nirikit.homeModules.default ];

  programs.nirikit = {
    enable = true;
    settings = {
      profiles.term3 = {
        workspace = "3";
        no-focus = true;
        silent = true;
        command = [ "kitty" ];
      };
    };
  };
}
```

### Flags

| Flag | Description |
|------|-------------|
| `-w, --workspace` | Target workspace (name or numeric index) |
| `--workspace-id` | Target workspace by niri ID |
| `-o, --output` | Restrict numeric index to this output |
| `--position` | Column placement: integer, `first`, or `last` |
| `-n, --no-focus` | Keep focus on the current window |
| `-s, --silent` | Suppress application stdout/stderr |
| `-t, --track-workspace` | Lock window to the workspace focused at launch time |
| `--match-app-id` | Override app ID for window matching |
| `--timeout` | Window wait timeout in seconds (default 10) |

### No-focus and niri limitations

`--no-focus` removes activation tokens, moves with `focus=false`, and reasserts the original window. For a truly invisible launch, enable niri's strict policy:

```kdl
debug { strict-new-window-focus-policy }
```

`position` requires briefly focusing the new window to use niri's `MoveColumnTo*` actions; focus is restored afterward. See [niri issue #915](https://github.com/niri-wm/niri/issues/915) for upstream per-launch rule support.

## Profiles

```toml
[profiles.term3]
workspace = "3"
no-focus = true
silent = true
command = ["kitty", "--class", "nirikit-kitty"]

[profiles.term1]
workspace = "3"
position = "first"
command = ["kitty"]

# Multi-window profiles.
# Use 01-, 02- prefixes to control launch order.
[profiles.dev]
workspace = "dev"
no-focus = true
silent = true

[profiles.dev.01-editor]
command = ["kitty", "--class", "nirikit-editor"]

[profiles.dev.02-browser]
command = ["firefox", "--new-window"]
workspace = "web"
position = "last"

[profiles.dev.03-chat]
command = ["signal-desktop"]
no-focus = false
```

Run a profile:

```console
nirikit profile dev
nirikit profile              # list profiles
```

Named child tables (`[profiles.NAME.WINDOW]`) accept the same fields and override profile defaults per-window.
