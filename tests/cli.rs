use clap::Parser;
use nirikit::cli::{Cli, Command};
use nirikit::profile::Position;

#[test]
fn parses_launch_and_preserves_command_arguments() {
    let cli = Cli::try_parse_from([
        "nirikit",
        "launch",
        "-w",
        "3",
        "--no-focus",
        "--silent",
        "--track-workspace",
        "--position",
        "last",
        "--",
        "kitty",
        "--class",
        "work",
    ])
    .unwrap();

    let args = match cli.command {
        Command::Launch(a) => a,
        _ => panic!("expected Launch"),
    };
    assert_eq!(args.workspace.as_deref(), Some("3"));
    assert!(args.no_focus);
    assert!(args.silent);
    assert!(args.track_workspace);
    assert_eq!(args.position, Some(Position::Last));
    assert_eq!(args.command, ["kitty", "--class", "work"]);
}

#[test]
fn parses_profile_subcommand() {
    let cli = Cli::try_parse_from(["nirikit", "profile", "term3", "--silent=false"]).unwrap();

    let args = match cli.command {
        Command::Profile(a) => a,
        _ => panic!("expected Profile"),
    };
    assert_eq!(args.name.as_deref(), Some("term3"));
    assert_eq!(args.overrides.silent, Some(false));
}

#[test]
fn parses_profile_subcommand_without_name() {
    let cli = Cli::try_parse_from(["nirikit", "profile"]).unwrap();

    let args = match cli.command {
        Command::Profile(a) => a,
        _ => panic!("expected Profile"),
    };
    assert!(args.name.is_none());
}

#[test]
fn rejects_non_positive_timeout() {
    assert!(Cli::try_parse_from(["nirikit", "launch", "--timeout", "0", "--", "kitty"]).is_err());
}
