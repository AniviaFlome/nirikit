use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: u64,
    pub idx: u8,
    pub name: Option<String>,
    pub output: Option<String>,
    #[serde(default)]
    pub is_focused: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Window {
    pub id: u64,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub is_focused: bool,
}

pub fn resolve_workspace(
    workspaces: &[Workspace],
    reference: Option<&str>,
    id: Option<u64>,
    output: Option<&str>,
) -> Result<Option<Workspace>> {
    if let Some(id) = id {
        return workspaces
            .iter()
            .find(|workspace| workspace.id == id)
            .cloned()
            .map(Some)
            .with_context(|| format!("workspace ID {id} does not exist"));
    }

    let Some(reference) = reference else {
        return Ok(None);
    };

    if let Ok(index) = reference.parse::<u8>() {
        let mut candidates: Vec<_> = workspaces
            .iter()
            .filter(|workspace| workspace.idx == index)
            .filter(|workspace| output.is_none() || workspace.output.as_deref() == output)
            .cloned()
            .collect();

        if candidates.len() > 1 && output.is_none() {
            if let Some(focused_output) = workspaces
                .iter()
                .find(|workspace| workspace.is_focused)
                .and_then(|workspace| workspace.output.as_deref())
            {
                let on_focused_output: Vec<_> = candidates
                    .iter()
                    .filter(|workspace| workspace.output.as_deref() == Some(focused_output))
                    .cloned()
                    .collect();
                if on_focused_output.len() == 1 {
                    candidates = on_focused_output;
                }
            }
        }

        return match candidates.as_slice() {
            [workspace] => Ok(Some(workspace.clone())),
            [] => bail!(
                "workspace index {index} does not exist{}",
                output
                    .map(|name| format!(" on output {name}"))
                    .unwrap_or_default()
            ),
            _ => bail!(
                "workspace index {index} exists on multiple outputs; pass --output or --workspace-id"
            ),
        };
    }

    workspaces
        .iter()
        .find(|workspace| workspace.name.as_deref() == Some(reference))
        .cloned()
        .map(Some)
        .with_context(|| format!("named workspace {reference:?} does not exist"))
}
