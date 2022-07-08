use denote::{cli, Error, NotesRepository, Result};
use std::{path::PathBuf, str::FromStr};

fn main() -> Result<()> {
    let base_path = PathBuf::from_str("notes").expect("'notes' is valid utf-8");
    let notes = NotesRepository::open(&base_path)?;

    let relative_path = PathBuf::from_str("2022/0708T174445--4__aso.md").unwrap();
    notes.on_update(&relative_path)
}
