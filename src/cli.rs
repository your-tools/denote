use crate::Result;
use std::{path::Path, process::Command};
use time::macros::format_description;
use time::OffsetDateTime;

use crate::{IOError, Notes};

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
        .map_err(|e| IOError(format!("Could not create temporary directory: {e}")))?;

    let note_path = temp_dir.path().join("note.md");
    std::fs::write(&note_path, template)
        .map_err(|e| IOError(format!("Could not create makdown file: {e}")))?;

    let status = Command::new("kak")
        .args([&note_path.as_os_str()])
        .status()
        .map_err(|e| IOError(format!("Could not spawn kakoune: {e}")))?;

    if !status.success() {
        return Err(IOError("kakoune did not exit sucessfully".to_string()))?;
    }
    if !&note_path.exists() {
        return Err(IOError(
            "kakoune exited successfuly but no file was written".to_string(),
        ))?;
    }

    let notes = Notes::try_new(&base_path)?;

    notes.import_from_markdown(&note_path)
}
