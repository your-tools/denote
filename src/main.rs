use denote::{NotesRepository, Result};
use std::{path::PathBuf, str::FromStr};

fn main() -> Result<()> {
    let base_path = PathBuf::from_str("notes").expect("'notes' is valid utf-8");
    let _notes = NotesRepository::open(&base_path)?;
    Ok(())
}
