/*
 * Copyright (C) 2016-2019 Federico Dossena
 * NoteBot Rust port: copyright is held by its respective contributors.
 *
 * This file is part of NoteBot and is distributed under the GNU GPL version 3.
 */
use crate::model::Note;
use directories::ProjectDirs;
use std::{fs, io, path::PathBuf};

fn data_file() -> io::Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "NoteBot", "NoteBot").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "user data directory unavailable")
    })?;
    Ok(dirs.data_local_dir().join("notes.json"))
}

pub fn load() -> io::Result<Vec<Note>> {
    let path = data_file()?;
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn save(notes: &[Note]) -> io::Result<()> {
    let path = data_file()?;
    let parent = path.parent().expect("data file has a parent directory");
    fs::create_dir_all(parent)?;

    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let bytes = serde_json::to_vec_pretty(notes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(&temporary, bytes)?;

    if path.exists() {
        let _ = fs::copy(&path, &backup);
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)
}
