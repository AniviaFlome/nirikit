use nirikit::model::{resolve_workspace, Workspace};

fn workspace(id: u64, idx: u8, name: Option<&str>, output: &str, focused: bool) -> Workspace {
    Workspace {
        id,
        idx,
        name: name.map(str::to_owned),
        output: Some(output.to_owned()),
        is_focused: focused,
    }
}

#[test]
fn resolves_name_and_id() {
    let workspaces = [workspace(12, 3, Some("chat"), "DP-1", true)];
    assert_eq!(
        resolve_workspace(&workspaces, Some("chat"), None, None)
            .unwrap()
            .unwrap()
            .id,
        12
    );
    assert_eq!(
        resolve_workspace(&workspaces, None, Some(12), None)
            .unwrap()
            .unwrap()
            .idx,
        3
    );
}

#[test]
fn numeric_index_prefers_focused_output() {
    let workspaces = [
        workspace(10, 3, None, "DP-1", false),
        workspace(20, 3, None, "DP-2", true),
    ];
    assert_eq!(
        resolve_workspace(&workspaces, Some("3"), None, None)
            .unwrap()
            .unwrap()
            .id,
        20
    );
}

#[test]
fn output_disambiguates_numeric_index() {
    let workspaces = [
        workspace(10, 3, None, "DP-1", false),
        workspace(20, 3, None, "DP-2", false),
    ];
    assert_eq!(
        resolve_workspace(&workspaces, Some("3"), None, Some("DP-1"))
            .unwrap()
            .unwrap()
            .id,
        10
    );
}

#[test]
fn ambiguous_index_is_rejected() {
    let workspaces = [
        workspace(10, 3, None, "DP-1", false),
        workspace(20, 3, None, "DP-2", false),
    ];
    assert!(resolve_workspace(&workspaces, Some("3"), None, None).is_err());
}
