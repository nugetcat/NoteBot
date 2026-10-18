/*
 * Derived from NoteBot by Federico Dossena, Copyright (C) 2016-2019.
 * Rust port copyright is held by its respective contributors.
 * This file is distributed under GNU GPL version 3 only.
 */
use crate::model::Note;
use directories::ProjectDirs;
use std::{fs, io, path::PathBuf};

fn data_file() -> io::Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "NoteBot").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "user data directory unavailable")
    })?;
    Ok(dirs.data_local_dir().join("notes.json"))
}

fn read_file(path: &PathBuf) -> io::Result<Option<Vec<Note>>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn load() -> io::Result<Vec<Note>> {
    let path = data_file()?;
    match read_file(&path) {
        Ok(Some(notes)) => Ok(notes),
        Ok(None) => match read_file(&path.with_extension("json.bak"))? {
            Some(notes) => Ok(notes),
            None => Ok(Vec::new()),
        },
        Err(primary_error) => match read_file(&path.with_extension("json.bak")) {
            Ok(Some(notes)) => Ok(notes),
            Ok(None) | Err(_) => Err(primary_error),
        },
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
