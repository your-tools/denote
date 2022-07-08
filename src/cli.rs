use crate::Result;
use std::{path::Path, process::Command};
use time::macros::format_description;
use time::OffsetDateTime;

use crate::{NotesRepository, OSError};

pub fn new_note(base_path: &Path) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let format = format_description!("[year][month][day]T[hour]:[minute]:[second]");
    let formatted_date = now
        .format(&format)
        .expect("format checked during compilation");

    let template = format!(
        r#"---
date: {formatted_date}
title:
keywords: 
---
    "#
    );
    let temp_dir = tempfile::Builder::new()
        .prefix("tmp-denotes")
        .tempdir()
        .map_err(|e| OSError(format!("Could not create temporary directory: {e}")))?;

    let note_path = temp_dir.path().join("note.md");
    std::fs::write(&note_path, template)
        .map_err(|e| OSError(format!("Could not create makdown file: {e}")))?;

    let editor = std::env::var("EDITOR").map_err(|_| OSError("EDITOR should be set".to_string()))?;

    let status = Command::new(&editor)
        .args([&note_path.as_os_str()])
        .status()
        .map_err(|e| OSError(format!("Could not spawn {editor}: {e}")))?;

    if !status.success() {
        return Err(OSError("editor did not exit sucessfully".to_string()))?;
    }
    if !&note_path.exists() {
        return Err(OSError(
            "editor exited successfuly but no file was written".to_string(),
        ))?;
    }

    let notes = NotesRepository::open(&base_path)?;

    notes.import_from_markdown(&note_path)
}
