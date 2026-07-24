use std::collections::BTreeMap;

use nirikit::profile::{
    merge, to_launch_args, CommandOverrides, Position, Profile, ProfileOverrides, ProfilesConfig,
};

fn parse_position(toml_value: &str) -> Result<Position, toml::de::Error> {
    toml::from_str::<Profile>(toml_value).and_then(|p| {
        p.position
            .ok_or_else(|| serde::de::Error::custom("no position"))
    })
}

#[test]
fn parses_numeric_position() {
    assert_eq!(
        parse_position("position = 2\n").unwrap(),
        Position::Index(2)
    );
}

#[test]
fn parses_first_and_last() {
    assert_eq!(
        parse_position("position = \"first\"\n").unwrap(),
        Position::First
    );
    assert_eq!(
        parse_position("position = \"last\"\n").unwrap(),
        Position::Last
    );
}

#[test]
fn rejects_zero_position() {
    assert!(parse_position("position = 0\n").is_err());
}

#[test]
fn rejects_unknown_position_string() {
    assert!(parse_position("position = \"middle\"\n").is_err());
}

#[test]
fn parses_full_profile() {
    let toml = r#"
workspace = "3"
no-focus = true
silent = true
track-workspace = true
position = "last"
command = ["kitty", "--class", "x"]
"#;
    let p: Profile = toml::from_str(toml).unwrap();
    assert_eq!(p.workspace.as_deref(), Some("3"));
    assert!(p.no_focus);
    assert!(p.silent);
    assert!(p.track_workspace);
    assert_eq!(p.position, Some(Position::Last));
    assert_eq!(p.command, ["kitty", "--class", "x"]);
}

#[test]
fn parses_config_with_multiple_profiles() {
    let toml = r#"
[profiles.term3]
workspace = "3"
command = ["kitty"]

[profiles.term4-right]
workspace = "4"
position = "last"
no-focus = true
command = ["kitty"]
"#;
    let c: ProfilesConfig = toml::from_str(toml).unwrap();
    assert!(c.profiles.contains_key("term3"));
    assert!(c.profiles.contains_key("term4-right"));
    assert_eq!(c.profiles["term4-right"].position, Some(Position::Last));
}

#[test]
fn allows_profile_named_list() {
    let toml = r#"
[profiles.list]
command = ["kitty"]
"#;
    let c: ProfilesConfig = toml::from_str(toml).unwrap();
    assert!(c.profiles.contains_key("list"));
}

#[test]
fn merge_overrides_replace_file_values() {
    let file = Profile {
        workspace: Some("3".to_owned()),
        no_focus: true,
        silent: true,
        track_workspace: true,
        command: vec!["kitty".to_owned()],
        ..Default::default()
    };
    let overrides = ProfileOverrides {
        no_focus: Some(false),
        track_workspace: Some(false),
        command: vec!["foot".to_owned()],
        ..Default::default()
    };
    let merged = merge(file, &overrides).unwrap();
    assert_eq!(merged.workspace.as_deref(), Some("3"));
    assert!(!merged.no_focus);
    assert!(merged.silent);
    assert!(!merged.track_workspace);
    assert_eq!(merged.command, ["foot"]);
}

#[test]
fn merge_position_first_last_integer() {
    let file = Profile::default();
    let overrides = ProfileOverrides {
        position: Some("first".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        merge(file.clone(), &overrides).unwrap().position,
        Some(Position::First)
    );

    let overrides = ProfileOverrides {
        position: Some("3".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        merge(file.clone(), &overrides).unwrap().position,
        Some(Position::Index(3))
    );

    let overrides = ProfileOverrides {
        position: Some("last".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        merge(file, &overrides).unwrap().position,
        Some(Position::Last)
    );
}

#[test]
fn to_launch_args_errors_without_command() {
    let p = Profile {
        command: vec![],
        ..Default::default()
    };
    assert!(to_launch_args(&p, None).is_err());
}

#[test]
fn to_launch_args_multi_windows() {
    let mut windows = BTreeMap::new();
    windows.insert(
        "01-editor".to_owned(),
        CommandOverrides {
            command: vec!["kitty".to_owned()],
            ..Default::default()
        },
    );
    windows.insert(
        "02-terminal".to_owned(),
        CommandOverrides {
            command: vec!["foot".to_owned()],
            ..Default::default()
        },
    );
    let p = Profile {
        windows,
        ..Default::default()
    };
    let args = to_launch_args(&p, None).unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].command, ["kitty"]);
    assert_eq!(args[1].command, ["foot"]);
}

#[test]
fn to_launch_args_per_window_overrides() {
    let mut windows = BTreeMap::new();
    windows.insert(
        "01-editor".to_owned(),
        CommandOverrides {
            command: vec!["kitty".to_owned()],
            ..Default::default()
        },
    );
    windows.insert(
        "02-browser".to_owned(),
        CommandOverrides {
            command: vec!["firefox".to_owned()],
            workspace: Some("web".to_owned()),
            position: Some(Position::Last),
            no_focus: Some(false),
            ..Default::default()
        },
    );
    let p = Profile {
        workspace: Some("3".to_owned()),
        no_focus: true,
        track_workspace: true,
        windows,
        ..Default::default()
    };
    let args = to_launch_args(&p, None).unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].workspace.as_deref(), Some("3"));
    assert!(args[0].no_focus);
    assert!(args[0].track_workspace);
    assert_eq!(args[0].command, ["kitty"]);
    assert_eq!(args[1].workspace.as_deref(), Some("web"));
    assert_eq!(args[1].position, Some(Position::Last));
    assert!(!args[1].no_focus);
    assert_eq!(args[1].command, ["firefox"]);
}

#[test]
fn parses_named_child_tables() {
    let toml = r#"
[profiles.test]
workspace = "3"
no-focus = true

[profiles.test.editor]
command = ["kitty", "--class", "x"]

[profiles.test.browser]
command = ["firefox"]
workspace = "web"
position = "last"
"#;
    let c: ProfilesConfig = toml::from_str(toml).unwrap();
    let p = &c.profiles["test"];
    assert_eq!(p.workspace.as_deref(), Some("3"));
    assert!(p.no_focus);
    assert_eq!(p.windows.len(), 2);
    let browser = &p.windows["browser"];
    assert_eq!(browser.command, ["firefox"]);
    assert_eq!(browser.workspace.as_deref(), Some("web"));
    assert_eq!(browser.position, Some(Position::Last));
    let editor = &p.windows["editor"];
    assert_eq!(editor.command, ["kitty", "--class", "x"]);
    assert!(editor.workspace.is_none());
}
