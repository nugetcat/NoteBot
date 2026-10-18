/*
 * Derived from NoteBot by Federico Dossena, Copyright (C) 2016-2019.
 * Rust port copyright is held by its respective contributors.
 * This file is distributed under GNU GPL version 3 only.
 */
mod model;
mod storage;

use model::{Note, PALETTE, next_id};
use slint::{ComponentHandle, SharedString};
use std::{cell::RefCell, rc::Rc};

slint::include_modules!();

type SharedNotes = Rc<RefCell<Vec<Note>>>;
type OpenWindows = Rc<RefCell<Vec<NoteWindow>>>;

fn persist(notes: &SharedNotes) {
    if let Err(error) = storage::save(&notes.borrow()) {
        eprintln!("NoteBot: could not save notes: {error}");
    }
}

fn open_note(
    note: Note,
    notes: SharedNotes,
    windows: OpenWindows,
) -> Result<(), slint::PlatformError> {
    let window = NoteWindow::new()?;
    let weak_window = window.as_weak();
    let note_id = note.id;

    window.set_note_text(note.text.clone().into());
    let color = PALETTE[note.color % PALETTE.len()];
    window.set_paper_color(slint::Color::from_rgb_u8(color[0], color[1], color[2]));
    window.set_text_scale(note.text_scale.clamp(0.2, 4.0));

    if !notes.borrow().iter().any(|existing| existing.id == note_id) {
        notes.borrow_mut().push(note.clone());
    }

    {
        let notes = notes.clone();
        window.on_note_text_edited(move |text: SharedString| {
            if let Some(item) = notes
                .borrow_mut()
                .iter_mut()
                .find(|item| item.id == note_id)
            {
                item.text = text.to_string();
            }
            persist(&notes);
        });
    }
    {
        let notes = notes.clone();
        let windows = windows.clone();
        window.on_new_note(move || {
            let id = next_id(&notes.borrow());
            if let Err(error) = open_note(Note::new(id), notes.clone(), windows.clone()) {
                eprintln!("NoteBot: could not open a note: {error}");
            }
        });
    }
    {
        let notes = notes.clone();
        window.on_delete_note(move || {
            notes.borrow_mut().retain(|item| item.id != note_id);
            persist(&notes);
            if let Some(window) = weak_window.upgrade() {
                let _ = window.hide();
            }
        });
    }
    {
        let notes = notes.clone();
        let weak_window = window.as_weak();
        window.on_next_color(move || {
            let mut next = 0;
            if let Some(item) = notes
                .borrow_mut()
                .iter_mut()
                .find(|item| item.id == note_id)
            {
                item.color = (item.color + 1) % PALETTE.len();
                next = item.color;
            }
            if let Some(window) = weak_window.upgrade() {
                let color = PALETTE[next];
                window.set_paper_color(slint::Color::from_rgb_u8(color[0], color[1], color[2]));
            }
            persist(&notes);
        });
    }
    for zoom_in in [true, false] {
        let notes = notes.clone();
        let weak_window = window.as_weak();
        let callback = move || {
            let mut scale = 1.0;
            if let Some(item) = notes
                .borrow_mut()
                .iter_mut()
                .find(|item| item.id == note_id)
            {
                item.text_scale =
                    (item.text_scale + if zoom_in { 0.1 } else { -0.1 }).clamp(0.2, 4.0);
                scale = item.text_scale;
            }
            if let Some(window) = weak_window.upgrade() {
                window.set_text_scale(scale);
            }
            persist(&notes);
        };
        if zoom_in {
            window.on_zoom_in(callback);
        } else {
            window.on_zoom_out(callback);
        }
    }

    window.show()?;
    windows.borrow_mut().push(window);
    persist(&notes);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = storage::load().unwrap_or_else(|error| {
        eprintln!("NoteBot: could not load saved notes: {error}");
        Vec::new()
    });
    let notes = Rc::new(RefCell::new(loaded));
    if notes.borrow().is_empty() {
        notes.borrow_mut().push(Note::new(1));
    }
    let windows = Rc::new(RefCell::new(Vec::new()));

    for note in notes.borrow().clone() {
        open_note(note, notes.clone(), windows.clone())?;
    }
    persist(&notes);
    slint::run_event_loop()?;
    Ok(())
}
