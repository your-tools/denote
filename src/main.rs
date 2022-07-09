use clap::Parser;
use denote::{cli, NotesRepository, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(version)]
struct Opts {
    #[clap(long, help = "Path of the notes repository")]
    base_path: PathBuf,
    #[clap(subcommand)]
    action: Action,
}

#[derive(Parser)]
enum Action {
    #[clap(about = "Create a new note from scratch")]
    Create,
    #[clap(about = "Update an existing note, renaming it if required")]
    Update(UpdateOpts),
}

#[derive(Parser)]
struct UpdateOpts {
    #[clap(help = "Path of the notes repository")]
    full_path: PathBuf,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let notes = NotesRepository::open(&opts.base_path)?;
    match opts.action {
        Action::Create => {
            cli::new_note(&opts.base_path)?;
            Ok(())
        }
        Action::Update(update) => {
            let relative_path = pathdiff::diff_paths(&update.full_path, &opts.base_path)
                .ok_or_else(|| {
                    eprintln!("repository and update paths should be relative to each other");
                    std::process::exit(1);
                })?;
            notes.update(&relative_path)?;
            Ok(())
        }
    }
}
