use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::model::{Window, Workspace};

const IO_POLL_INTERVAL: Duration = Duration::from_millis(100);
const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct InitialState {
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
}

#[derive(Debug)]
pub enum Event {
    WindowsChanged(Vec<Window>),
    WindowOpenedOrChanged(Window),
    WindowClosed(u64),
    WindowFocusChanged(Option<u64>),
    Other,
}

pub struct NiriIpc {
    socket: PathBuf,
}

impl NiriIpc {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn event_stream(&self) -> Result<(EventStream, InitialState)> {
        EventStream::connect(&self.socket)
    }

    pub fn move_window_to_workspace(
        &self,
        window_id: u64,
        workspace_id: u64,
        focus: bool,
    ) -> Result<()> {
        self.action(json!({
            "MoveWindowToWorkspace": {
                "window_id": window_id,
                "reference": { "Id": workspace_id },
                "focus": focus
            }
        }))
    }

    pub fn focus_window(&self, id: u64) -> Result<()> {
        self.action(json!({ "FocusWindow": { "id": id } }))
    }

    /// Move the focused column to a specific 1-based index on its workspace.
    pub fn move_column_to_index(&self, index: u32) -> Result<()> {
        self.action(json!({ "MoveColumnToIndex": { "index": index } }))
    }

    /// Move the focused column to the first position on its workspace.
    pub fn move_column_to_first(&self) -> Result<()> {
        self.action(json!({ "MoveColumnToFirst": {} }))
    }

    /// Move the focused column to the last position on its workspace.
    pub fn move_column_to_last(&self) -> Result<()> {
        self.action(json!({ "MoveColumnToLast": {} }))
    }

    pub fn windows(&self) -> Result<Vec<Window>> {
        let response = self.request(json!("Windows"))?;
        let windows = response
            .get("Windows")
            .context("niri returned an unexpected response to Windows")?;
        serde_json::from_value(windows.clone()).context("could not decode niri windows")
    }

    fn action(&self, action: Value) -> Result<()> {
        let response = self.request(json!({ "Action": action }))?;
        if response == json!("Handled") || response.get("Handled").is_some() {
            Ok(())
        } else {
            bail!("niri returned an unexpected action response: {response}")
        }
    }

    fn request(&self, request: Value) -> Result<Value> {
        let mut stream = UnixStream::connect(&self.socket).with_context(|| {
            format!("could not connect to niri socket {}", self.socket.display())
        })?;
        writeln!(stream, "{}", serde_json::to_string(&request)?)
            .context("could not write to niri socket")?;

        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .context("could not read from niri socket")?;
        decode_reply(&line)
    }
}

pub struct EventStream {
    reader: BufReader<UnixStream>,
}

impl EventStream {
    fn connect(socket: &Path) -> Result<(Self, InitialState)> {
        let mut stream = UnixStream::connect(socket)
            .with_context(|| format!("could not connect to niri socket {}", socket.display()))?;
        stream
            .set_read_timeout(Some(IO_POLL_INTERVAL))
            .context("could not configure niri socket")?;
        writeln!(stream, "\"EventStream\"").context("could not request niri event stream")?;

        let mut event_stream = Self {
            reader: BufReader::new(stream),
        };
        let reply = event_stream
            .read_raw_until(Instant::now() + BOOTSTRAP_TIMEOUT)?
            .context("niri closed the event stream before acknowledging it")?;
        let response = decode_reply(&reply)?;
        if response != json!("Handled") && response.get("Handled").is_none() {
            bail!("niri returned an unexpected event-stream response: {response}");
        }

        let deadline = Instant::now() + BOOTSTRAP_TIMEOUT;
        let mut workspaces = None;
        let mut windows = None;
        while workspaces.is_none() || windows.is_none() {
            let raw = event_stream
                .read_raw_until(deadline)?
                .context("niri closed the event stream during initial state")?;
            let value: Value = serde_json::from_str(&raw).context("invalid niri event JSON")?;
            if let Some(payload) = value.get("WorkspacesChanged") {
                workspaces = Some(
                    serde_json::from_value(
                        payload
                            .get("workspaces")
                            .context("WorkspacesChanged event has no workspaces")?
                            .clone(),
                    )
                    .context("could not decode niri workspaces")?,
                );
            } else if let Some(payload) = value.get("WindowsChanged") {
                windows = Some(
                    serde_json::from_value(
                        payload
                            .get("windows")
                            .context("WindowsChanged event has no windows")?
                            .clone(),
                    )
                    .context("could not decode niri windows")?,
                );
            }
        }

        Ok((
            event_stream,
            InitialState {
                workspaces: workspaces.unwrap(),
                windows: windows.unwrap(),
            },
        ))
    }

    pub fn next_event_until(&mut self, deadline: Instant) -> Result<Option<Event>> {
        let Some(raw) = self.read_raw_until(deadline)? else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(&raw).context("invalid niri event JSON")?;
        parse_event(value).map(Some)
    }

    fn read_raw_until(&mut self, deadline: Instant) -> Result<Option<String>> {
        loop {
            if Instant::now() >= deadline {
                return Ok(None);
            }

            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return Ok(None),
                Ok(_) => return Ok(Some(line)),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => return Err(error).context("could not read niri event stream"),
            }
        }
    }
}

pub fn decode_reply(line: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(line).context("invalid niri reply JSON")?;
    if let Some(ok) = value.get("Ok") {
        return Ok(ok.clone());
    }
    if let Some(error) = value.get("Err") {
        bail!("niri rejected the request: {error}");
    }
    Err(anyhow!("unexpected niri reply: {value}"))
}

pub fn parse_event(value: Value) -> Result<Event> {
    if let Some(payload) = value.get("WindowsChanged") {
        let windows = serde_json::from_value(
            payload
                .get("windows")
                .context("WindowsChanged event has no windows")?
                .clone(),
        )
        .context("could not decode niri windows")?;
        return Ok(Event::WindowsChanged(windows));
    }
    if let Some(payload) = value.get("WindowOpenedOrChanged") {
        let window = serde_json::from_value(
            payload
                .get("window")
                .context("WindowOpenedOrChanged event has no window")?
                .clone(),
        )
        .context("could not decode niri window")?;
        return Ok(Event::WindowOpenedOrChanged(window));
    }
    if let Some(payload) = value.get("WindowClosed") {
        return Ok(Event::WindowClosed(
            payload
                .get("id")
                .and_then(Value::as_u64)
                .context("WindowClosed event has no ID")?,
        ));
    }
    if let Some(payload) = value.get("WindowFocusChanged") {
        return Ok(Event::WindowFocusChanged(
            payload.get("id").and_then(Value::as_u64),
        ));
    }
    Ok(Event::Other)
}
