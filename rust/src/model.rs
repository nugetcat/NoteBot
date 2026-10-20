/*
 * Copyright (C) 2016-2019 Federico Dossena
 * NoteBot Rust port: copyright is held by its respective contributors.
 *
 * This file is part of NoteBot and is distributed under the GNU GPL version 3.
 */
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub color: usize,
    #[serde(default)]
    pub custom_color: Option<String>,
    pub text_scale: f32,
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
}

impl Note {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            text: String::new(),
            color: 0,
            custom_color: None,
            text_scale: 1.0,
            x: None,
            y: None,
            width: None,
            height: None,
        }
    }
}

// Palette order and RGB values follow the original Java Note implementation.
pub const PALETTE: [[u8; 3]; 8] = [
    [253, 253, 202], // yellow note interior
    [255, 215, 173], // orange
    [217, 243, 251], // blue
    [209, 254, 203], // green
    [246, 211, 246], // pink
    [221, 217, 254], // purple
    [254, 209, 203], // red
    [255, 255, 255], // white
];

pub fn next_id(notes: &[Note]) -> u64 {
    notes.iter().map(|note| note.id).max().unwrap_or(0) + 1
}
