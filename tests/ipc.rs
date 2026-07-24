use nirikit::ipc::{decode_reply, parse_event, Event};
use serde_json::json;

#[test]
fn parses_window_open_event() {
    let event = parse_event(json!({
        "WindowOpenedOrChanged": {
            "window": {
                "id": 7,
                "app_id": "kitty",
                "pid": 123,
                "workspace_id": 2,
                "is_focused": true
            }
        }
    }))
    .unwrap();
    let Event::WindowOpenedOrChanged(window) = event else {
        panic!("unexpected event")
    };
    assert_eq!(window.id, 7);
    assert_eq!(window.pid, Some(123));
}

#[test]
fn decodes_errors() {
    let error = decode_reply(r#"{"Err":"bad action"}"#).unwrap_err();
    assert!(error.to_string().contains("bad action"));
}
