use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use serde::Deserialize;

use crate::cli::LaunchArgs;

/// Column placement for a launched window.
///
/// `1` is the first column (leftmost by default; rightmost when niri's layout
/// direction is inverted). Accepts an integer index, or the strings "first"
/// and "last".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    /// 1-based column index.
    Index(u32),
    /// First column (index 1).
    First,
    /// Last column on the workspace.
    Last,
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum IntOrStr {
            Int(u32),
            Str(String),
        }

        match IntOrStr::deserialize(deserializer)? {
            IntOrStr::Int(i) => {
                if i == 0 {
                    return Err(serde::de::Error::custom(
                        "position must be >= 1 (use \"first\" for column 1)",
                    ));
                }
                Ok(Position::Index(i))
            }
            IntOrStr::Str(s) => match s.as_str() {
                "first" => Ok(Position::First),
                "last" => Ok(Position::Last),
                other => Err(serde::de::Error::custom(format!(
                    "position string must be 'first' or 'last', got {other:?}"
                ))),
            },
        }
    }
}

impl FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "first" | "1" => Ok(Position::First),
            "last" => Ok(Position::Last),
            other => other
                .parse::<u32>()
                .map_err(|_| {
                    format!("position must be an integer, \"first\", or \"last\" (got {other:?})")
                })
                .and_then(|n| {
                    if n == 0 {
                        Err("position must be >= 1 (use \"first\" for column 1)".to_owned())
                    } else {
                        Ok(Position::Index(n))
                    }
                }),
        }
    }
}

/// A reusable launch configuration stored in the config file.
///
/// Profile-level fields apply to all windows. Named child tables
/// (`[profiles.NAME.WINDOW_NAME]`) define individual windows with per-window
/// overrides; fields not set in a child fall through to the profile defaults.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
    /// Target workspace name or numeric index.
    pub workspace: Option<String>,
    /// Globally unique niri workspace ID.
    pub workspace_id: Option<u64>,
    /// Restrict a numeric workspace index to this output.
    pub output: Option<String>,
    /// Column placement after the window opens.
    pub position: Option<Position>,
    /// Keep keyboard focus on the previously focused window.
    #[serde(default)]
    pub no_focus: bool,
    /// Suppress the launched application's stdout and stderr.
    #[serde(default)]
    pub silent: bool,
    /// Lock the window to the workspace that was focused at launch time.
    #[serde(default)]
    pub track_workspace: bool,
    /// Match the new window using this exact app ID.
    pub match_app_id: Option<String>,
    /// How long to wait for the application to create a window.
    pub timeout: Option<Duration>,
    /// A single command to launch. Ignored if named child tables exist.
    #[serde(default)]
    pub command: Vec<String>,
    /// Named child tables, each defining a window with per-window overrides.
    /// In TOML: `[profiles.dev.editor]`, `[profiles.dev.browser]`, etc.
    /// Order is alphabetical by name (use leading numbers like `01-`, `02-`
    /// to control launch order).
    #[serde(flatten)]
    pub windows: BTreeMap<String, CommandOverrides>,
}

/// Per-window configuration for a named child table of a profile.
///
/// Each `[profiles.NAME.WINDOW_NAME]` table produces one `LaunchArgs`.
/// Fields not set here fall through to the profile-level defaults.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct CommandOverrides {
    /// Command and arguments to launch. Required.
    pub command: Vec<String>,
    /// Override the target workspace for this window only.
    pub workspace: Option<String>,
    /// Override the target workspace ID for this window only.
    pub workspace_id: Option<u64>,
    /// Override the output filter for this window only.
    pub output: Option<String>,
    /// Override column placement for this window only.
    pub position: Option<Position>,
    /// Override no-focus for this window only.
    pub no_focus: Option<bool>,
    /// Override silent for this window only.
    pub silent: Option<bool>,
    /// Override track-workspace for this window only.
    pub track_workspace: Option<bool>,
    /// Override the app ID used for window matching.
    pub match_app_id: Option<String>,
    /// Override the window wait timeout for this window only.
    pub timeout: Option<Duration>,
}

/// Parsed configuration file.
#[derive(Debug, Deserialize, Default)]
pub struct ProfilesConfig {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

/// CLI overrides applied on top of a file-defined profile.
#[derive(Debug, Clone, Args, Default)]
pub struct ProfileOverrides {
    /// Override the target workspace.
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// Override the target workspace ID.
    #[arg(long)]
    pub workspace_id: Option<u64>,

    /// Override the output filter for numeric workspace indices.
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Override column placement. Accepts an integer, "first", or "last".
    #[arg(long)]
    pub position: Option<String>,

    /// Override no-focus. Use `--no-focus` to enable, `--no-focus=false` to disable.
    #[arg(
        short = 'n',
        long,
        action = clap::ArgAction::Set,
        default_missing_value = "true",
        num_args = 0..=1,
    )]
    pub no_focus: Option<bool>,

    /// Override silent. Use `--silent` to enable, `--silent=false` to disable.
    #[arg(
        short = 's',
        long,
        action = clap::ArgAction::Set,
        default_missing_value = "true",
        num_args = 0..=1,
    )]
    pub silent: Option<bool>,

    /// Override track-workspace. Use `--track-workspace` to enable, `--track-workspace=false` to disable.
    #[arg(
        long,
        action = clap::ArgAction::Set,
        default_missing_value = "true",
        num_args = 0..=1,
    )]
    pub track_workspace: Option<bool>,

    /// Override the app ID used for window matching.
    #[arg(long)]
    pub match_app_id: Option<String>,

    /// Override the window wait timeout, in seconds.
    #[arg(long, value_parser = parse_duration_seconds)]
    pub timeout: Option<Duration>,

    /// Command and arguments to launch, overriding the profile's command.
    /// Place these after `--`.
    #[arg(last = true, num_args = 1..)]
    pub command: Vec<String>,
}

fn parse_duration_seconds(value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<f64>()
        .map_err(|_| "timeout must be a number of seconds".to_owned())?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("timeout must be greater than zero".to_owned());
    }
    Ok(Duration::from_secs_f64(seconds))
}

/// Resolve the config file path.
///
/// Precedence: explicit `override` > `$NIRIKIT_CONFIG` > `$XDG_CONFIG_HOME/nirikit/config.toml`
/// > `~/.config/nirikit/config.toml`.
pub fn find_config_path(override_path: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return Some(p);
    }
    if let Ok(p) = std::env::var("NIRIKIT_CONFIG") {
        return Some(PathBuf::from(p));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("nirikit").join("config.toml");
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::config_dir() {
        let p = home.join("nirikit").join("config.toml");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Describe all search locations, for use in error messages.
fn searched_locations() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("NIRIKIT_CONFIG") {
        out.push(format!("$NIRIKIT_CONFIG ({p})"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        out.push(format!(
            "$XDG_CONFIG_HOME/nirikit/config.toml ({})",
            PathBuf::from(xdg)
                .join("nirikit")
                .join("config.toml")
                .display()
        ));
    }
    if let Some(home) = dirs::config_dir() {
        out.push(format!(
            "{}/nirikit/config.toml ({})",
            home.display(),
            home.join("nirikit").join("config.toml").display()
        ));
    }
    out
}

/// Load and parse a config file.
pub fn load(path: &Path) -> Result<ProfilesConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("could not read config file {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("could not parse config file {}", path.display()))
}

/// Merge a file-defined profile with CLI overrides. CLI wins where present.
pub fn merge(file: Profile, overrides: &ProfileOverrides) -> Result<Profile> {
    let position = match overrides.position.as_deref() {
        Some("first") | Some("1") => Some(Position::First),
        Some("last") => Some(Position::Last),
        Some(other) => match other.parse::<u32>() {
            Ok(0) => bail!("--position must be >= 1 (use \"first\" for column 1)"),
            Ok(n) => Some(Position::Index(n)),
            Err(_) => {
                bail!("--position must be an integer, \"first\", or \"last\" (got {other:?})")
            }
        },
        None => file.position,
    };

    Ok(Profile {
        workspace: overrides.workspace.clone().or(file.workspace),
        workspace_id: overrides.workspace_id.or(file.workspace_id),
        output: overrides.output.clone().or(file.output),
        position,
        no_focus: overrides.no_focus.unwrap_or(file.no_focus),
        silent: overrides.silent.unwrap_or(file.silent),
        track_workspace: overrides.track_workspace.unwrap_or(file.track_workspace),
        match_app_id: overrides.match_app_id.clone().or(file.match_app_id),
        timeout: overrides.timeout.or(file.timeout),
        command: if overrides.command.is_empty() {
            file.command
        } else {
            overrides.command.clone()
        },
        // CLI override replaces the whole window set.
        windows: if overrides.command.is_empty() {
            file.windows
        } else {
            BTreeMap::new()
        },
    })
}

/// Convert a merged profile into one or more `LaunchArgs` for `launch::run`.
///
/// A profile with named child tables produces one `LaunchArgs` per child,
/// with each child's per-window overrides applied on top of the profile
/// defaults. A profile with only `command` produces a single `LaunchArgs`.
/// CLI override (a non-empty `command` field on the merged profile) always
/// wins and produces a single `LaunchArgs`.
pub fn to_launch_args(profile: &Profile, socket: Option<PathBuf>) -> Result<Vec<LaunchArgs>> {
    let build = |ov: &CommandOverrides| -> Result<LaunchArgs> {
        if ov.command.is_empty() {
            bail!("profile has a window entry with no 'command'");
        }
        Ok(LaunchArgs {
            workspace: ov.workspace.clone().or(profile.workspace.clone()),
            workspace_id: ov.workspace_id.or(profile.workspace_id),
            output: ov.output.clone().or(profile.output.clone()),
            position: ov.position.clone().or(profile.position.clone()),
            no_focus: ov.no_focus.unwrap_or(profile.no_focus),
            silent: ov.silent.unwrap_or(profile.silent),
            track_workspace: ov.track_workspace.unwrap_or(profile.track_workspace),
            match_app_id: ov.match_app_id.clone().or(profile.match_app_id.clone()),
            timeout: ov
                .timeout
                .or(profile.timeout)
                .unwrap_or_else(|| Duration::from_secs(10)),
            socket: socket.clone(),
            command: ov.command.clone(),
        })
    };

    if !profile.command.is_empty() {
        // Single command: wrap in a minimal CommandOverrides.
        return Ok(vec![build(&CommandOverrides {
            command: profile.command.clone(),
            ..Default::default()
        })?]);
    }
    if profile.windows.is_empty() {
        bail!(
            "profile has no 'command' or named child tables and no command was passed on the CLI"
        );
    }
    // BTreeMap iterates in sorted key order; use leading numbers like
    // `01-editor`, `02-browser` to control launch order.
    profile.windows.values().map(build).collect()
}

/// Run the `profile` subcommand.
///
/// When `name` is `None` (no positional argument), list defined profiles.
/// Otherwise, run the named profile, applying any CLI overrides.
pub fn run(
    name: Option<String>,
    config: Option<PathBuf>,
    overrides: ProfileOverrides,
) -> Result<()> {
    let path = find_config_path(config).ok_or_else(|| {
        anyhow!(
            "could not find nirikit config; searched: {}",
            searched_locations().join(", ")
        )
    })?;
    let parsed = load(&path)?;

    let Some(name) = name else {
        let mut names: Vec<&String> = parsed.profiles.keys().collect();
        names.sort();
        if names.is_empty() {
            println!("(no profiles defined in {})", path.display());
        } else {
            for n in names {
                println!("{n}");
            }
        }
        return Ok(());
    };

    let file_profile = parsed
        .profiles
        .get(&name)
        .ok_or_else(|| {
            let mut available: Vec<&String> = parsed.profiles.keys().collect();
            available.sort();
            anyhow!(
                "profile {name:?} not found; available: {}",
                if available.is_empty() {
                    "(none)".to_owned()
                } else {
                    available
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            )
        })?
        .clone();

    let merged = merge(file_profile, &overrides)?;
    // Profiles construct LaunchArgs programmatically, so clap's `env = "NIRI_SOCKET"`
    // resolution never runs. Resolve it manually here.
    let socket = std::env::var_os("NIRI_SOCKET").map(PathBuf::from);
    let args_list = to_launch_args(&merged, socket)?;
    for args in args_list {
        crate::launch::run(args)?;
    }
    Ok(())
}
