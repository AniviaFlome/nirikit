use anyhow::Result;
use clap::Parser;
use nirikit::cli::{Cli, Command};

fn main() {
    if let Err(error) = try_main() {
        eprintln!("nirikit: {error:#}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Launch(args) => nirikit::launch::run(args),
        Command::Profile(args) => nirikit::profile::run(args.name, args.config, args.overrides),
    }
}
