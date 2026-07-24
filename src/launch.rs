use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::cli::LaunchArgs;
use crate::ipc::{Event, EventStream, NiriIpc};
use crate::model::{resolve_workspace, Window};
use crate::profile::Position;

const FALLBACK_MATCH_DELAY: Duration = Duration::from_millis(300);

pub fn run(args: LaunchArgs) -> Result<()> {
    let socket = args
        .socket
        .as_ref()
        .context("NIRI_SOCKET is not set; run nirikit inside a niri session")?;
    let ipc = NiriIpc::new(socket);
    let (mut events, initial) = ipc.event_stream()?;

    // When --track-workspace is set and no explicit workspace target was given,
    // capture the currently-focused workspace at launch time so the window lands
    // there even if the user switches workspaces before it opens.
    let tracked_workspace =
        if args.track_workspace && args.workspace.is_none() && args.workspace_id.is_none() {
            initial
                .workspaces
                .iter()
                .find(|ws| ws.is_focused)
                .map(|ws| (ws.id, ws.idx))
        } else {
            None
        };

    let target = resolve_workspace(
        &initial.workspaces,
        args.workspace.as_deref(),
        args.workspace_id,
        args.output.as_deref(),
    )?;
    let original_focus = initial
        .windows
        .iter()
        .find(|window| window.is_focused)
        .map(|window| window.id);
    let baseline: HashSet<_> = initial.windows.iter().map(|window| window.id).collect();

    let executable = args.command.first().context("no launch command provided")?;
    let command_app_id = Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable)
        .to_owned();

    let mut command = Command::new(executable);
    command.args(&args.command[1..]).stdin(Stdio::null());
    if args.silent {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    if args.no_focus {
        // A tokenless window is not activated when niri's strict focus policy is enabled.
        command
            .env_remove("XDG_ACTIVATION_TOKEN")
            .env_remove("DESKTOP_STARTUP_ID");
    }
    let child = command
        .spawn()
        .with_context(|| format!("could not launch {executable:?}"))?;
    let root_pid = child.id() as i32;

    let window = wait_for_window(
        &mut events,
        &baseline,
        root_pid,
        &command_app_id,
        args.match_app_id.as_deref(),
        args.timeout,
    )?;

    // Resolve the effective workspace target: either the explicit one from
    // resolve_workspace, or the tracked workspace captured at launch time.
    let effective_target: Option<u64> = if let Some(ws) = target.as_ref() {
        Some(ws.id)
    } else if let Some((id, _)) = tracked_workspace {
        Some(id)
    } else {
        None
    };

    if let Some(workspace_id) = effective_target {
        if window.workspace_id != Some(workspace_id) {
            ipc.move_window_to_workspace(window.id, workspace_id, !args.no_focus)?;
        }
    }

    // Column placement requires the new window's column to be focused for the
    // MoveColumnTo* action. With --no-focus this causes a momentary focus shift
    // that is undone by restore_focus_if_needed below.
    if let Some(position) = args.position.as_ref() {
        ipc.focus_window(window.id)?;
        match position {
            Position::First => ipc.move_column_to_first()?,
            Position::Last => ipc.move_column_to_last()?,
            Position::Index(i) => ipc.move_column_to_index(*i)?,
        }
    }

    if args.no_focus {
        restore_focus_if_needed(&ipc, original_focus, window.id)?;
    } else if effective_target.is_none() && args.position.is_none() {
        // Make launch semantics deterministic even under strict focus policies,
        // but skip the explicit focus if placement already focused the window.
        ipc.focus_window(window.id)?;
    }

    Ok(())
}

fn wait_for_window(
    events: &mut EventStream,
    baseline: &HashSet<u64>,
    root_pid: i32,
    command_app_id: &str,
    explicit_app_id: Option<&str>,
    timeout: Duration,
) -> Result<Window> {
    let deadline = Instant::now() + timeout;
    let mut pending: HashMap<u64, (Window, Instant)> = HashMap::new();

    loop {
        if let Some(window) = select_fallback(&pending) {
            return Ok(window);
        }

        let poll_deadline = fallback_deadline(&pending)
            .map(|fallback| fallback.min(deadline))
            .unwrap_or(deadline);
        let Some(event) = events.next_event_until(poll_deadline)? else {
            if Instant::now() >= deadline {
                bail!(
                    "timed out after {:.1}s waiting for the launched application to create a window",
                    timeout.as_secs_f64()
                );
            }
            continue;
        };

        match event {
            Event::WindowOpenedOrChanged(window) => {
                if baseline.contains(&window.id) {
                    continue;
                }
                if is_strong_match(&window, root_pid, command_app_id, explicit_app_id) {
                    return Ok(window);
                }
                pending.insert(window.id, (window, Instant::now()));
            }
            Event::WindowsChanged(windows) => {
                for window in windows {
                    if baseline.contains(&window.id) {
                        continue;
                    }
                    if is_strong_match(&window, root_pid, command_app_id, explicit_app_id) {
                        return Ok(window);
                    }
                    pending.entry(window.id).or_insert((window, Instant::now()));
                }
            }
            Event::WindowClosed(id) => {
                pending.remove(&id);
            }
            Event::WindowFocusChanged(id) => {
                let _ = id;
            }
            Event::Other => {}
        }
    }
}

pub fn is_strong_match(
    window: &Window,
    root_pid: i32,
    command_app_id: &str,
    explicit_app_id: Option<&str>,
) -> bool {
    if let Some(expected) = explicit_app_id {
        return window.app_id.as_deref() == Some(expected);
    }

    if window
        .pid
        .is_some_and(|pid| process_descends_from(pid, root_pid))
    {
        return true;
    }

    window.app_id.as_deref().is_some_and(|app_id| {
        app_id.eq_ignore_ascii_case(command_app_id)
            || app_id
                .rsplit('.')
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case(command_app_id))
    })
}

fn select_fallback(pending: &HashMap<u64, (Window, Instant)>) -> Option<Window> {
    if pending.len() != 1 {
        return None;
    }
    let (window, seen_at) = pending.values().next()?;
    (seen_at.elapsed() >= FALLBACK_MATCH_DELAY).then(|| window.clone())
}

fn fallback_deadline(pending: &HashMap<u64, (Window, Instant)>) -> Option<Instant> {
    if pending.len() != 1 {
        return None;
    }
    pending
        .values()
        .next()
        .map(|(_, seen_at)| *seen_at + FALLBACK_MATCH_DELAY)
}

fn restore_focus_if_needed(
    ipc: &NiriIpc,
    original_focus: Option<u64>,
    launched_id: u64,
) -> Result<()> {
    let Some(original_focus) = original_focus else {
        return Ok(());
    };
    let current = ipc
        .windows()?
        .into_iter()
        .find(|window| window.is_focused)
        .map(|window| window.id);
    // Reasserting the original focus also asks niri to scroll its column back into
    // view after the new rightmost window moved the viewport.
    if current == Some(launched_id) || current == Some(original_focus) {
        ipc.focus_window(original_focus)
            .context("the application launched, but restoring the previous focus failed")?;
    }
    Ok(())
}

pub fn process_descends_from(mut pid: i32, ancestor: i32) -> bool {
    let mut visited = HashSet::new();
    while pid > 1 && visited.insert(pid) {
        if pid == ancestor {
            return true;
        }
        let Some(parent) = parent_pid(pid) else {
            return false;
        };
        pid = parent;
    }
    false
}

fn parent_pid(pid: i32) -> Option<i32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    after_name.split_whitespace().nth(1)?.parse().ok()
}
