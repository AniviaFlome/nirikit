use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use crate::profile::Position;

#[derive(Debug, Parser)]
#[command(name = "nirikit", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Launch an application and optionally place its first window.
    Launch(LaunchArgs),
    /// Run a named profile from the config file.
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
pub struct LaunchArgs {
    /// Target workspace name or index. Numeric values are treated as indices.
    #[arg(short = 'w', long, conflicts_with = "workspace_id")]
    pub workspace: Option<String>,

    /// Target a workspace by its globally unique niri ID.
    #[arg(long, conflicts_with = "workspace")]
    pub workspace_id: Option<u64>,

    /// Restrict a numeric workspace index to this output.
    #[arg(short = 'o', long, requires = "workspace")]
    pub output: Option<String>,

    /// Column placement after the window opens. Accepts an integer (1-based), "first", or "last".
    #[arg(long)]
    pub position: Option<Position>,

    /// Keep keyboard focus on the previously focused window.
    #[arg(short = 'n', long)]
    pub no_focus: bool,

    /// Suppress the launched application's stdout and stderr.
    #[arg(short = 's', long)]
    pub silent: bool,

    /// Lock the window to the workspace that was focused when the command was launched.
    #[arg(short = 't', long)]
    pub track_workspace: bool,

    /// Match the new window using this exact app ID when PID matching is unavailable.
    #[arg(long, value_name = "APP_ID")]
    pub match_app_id: Option<String>,

    /// How long to wait for the application to create a window.
    #[arg(long, default_value = "10", value_parser = parse_duration_seconds)]
    pub timeout: Duration,

    /// Override the niri IPC socket (primarily useful for testing).
    #[arg(long, env = "NIRI_SOCKET", hide = true)]
    pub socket: Option<PathBuf>,

    /// Command and arguments to launch. Place these after `--`.
    #[arg(required = true, last = true, num_args = 1..)]
    pub command: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    /// Name of the profile to run. Omit to list defined profiles.
    pub name: Option<String>,

    /// Override the config file path (defaults to $XDG_CONFIG_HOME/nirikit/config.toml).
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub overrides: crate::profile::ProfileOverrides,
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
