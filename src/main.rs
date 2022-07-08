use denote::{cli, Error, Result};
use std::{path::PathBuf, str::FromStr};

fn main() -> Result<()> {
    let base_path = PathBuf::from_str("notes").expect("'notes' is valid utf-8");
    cli::new_note(&base_path)
}
