use nirikit::launch::{is_strong_match, process_descends_from};
use nirikit::model::Window;

fn window(id: u64, app_id: Option<&str>, pid: Option<i32>) -> Window {
    Window {
        id,
        app_id: app_id.map(str::to_owned),
        pid,
        workspace_id: Some(1),
        is_focused: false,
    }
}

#[test]
fn explicit_app_id_takes_precedence() {
    assert!(is_strong_match(
        &window(1, Some("org.example.App"), None),
        999_999,
        "app",
        Some("org.example.App")
    ));
    assert!(!is_strong_match(
        &window(1, Some("app"), Some(std::process::id() as i32)),
        std::process::id() as i32,
        "app",
        Some("different")
    ));
}

#[test]
fn command_name_matches_reverse_domain_app_id() {
    assert!(is_strong_match(
        &window(1, Some("org.example.kitty"), None),
        999_999,
        "kitty",
        None
    ));
}

#[test]
fn current_process_descends_from_itself() {
    let pid = std::process::id() as i32;
    assert!(process_descends_from(pid, pid));
}
