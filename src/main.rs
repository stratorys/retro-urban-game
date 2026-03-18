use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "retro-urban-game")]
#[command(about = "Retro Urban game runtime and map editor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Play(MapPathArgs),
    Edit(EditArgs),
}

#[derive(Args)]
struct MapPathArgs {
    #[arg(long, value_name = "PATH")]
    map: PathBuf,
}

#[derive(Args)]
struct EditArgs {
    #[arg(long, value_name = "PATH")]
    map: PathBuf,
    #[arg(long)]
    new: bool,
    #[arg(long, requires = "new")]
    width: Option<u32>,
    #[arg(long, requires = "new")]
    height: Option<u32>,
    #[arg(long, requires = "new")]
    overwrite: bool,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Play(args) => game_app::run(&args.map).map_err(|err| err.to_string()),
        Commands::Edit(args) => {
            let new_map = if args.new {
                let width = args.width.ok_or_else(|| {
                    "--width is required when --new is provided".to_owned()
                });
                let height = args.height.ok_or_else(|| {
                    "--height is required when --new is provided".to_owned()
                });
                Some(match (width, height) {
                    (Ok(width), Ok(height)) => map_editor::NewMapArgs { width, height },
                    (Err(error), _) | (_, Err(error)) => {
                        eprintln!("{error}");
                        std::process::exit(2);
                    }
                })
            } else {
                if args.width.is_some() || args.height.is_some() || args.overwrite {
                    eprintln!("--width/--height/--overwrite require --new");
                    std::process::exit(2);
                }
                None
            };

            map_editor::run(&args.map, new_map, args.overwrite).map_err(|err| err.to_string())
        }
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
