/*
 * Derived from NoteBot by Federico Dossena, Copyright (C) 2016-2019.
 * Rust port copyright is held by its respective contributors.
 * This file is distributed under GNU GPL version 3 only.
 */
mod model;
mod storage;

use model::{Note, PALETTE, next_id};
use slint::{ComponentHandle, SharedString, Timer, TimerMode};
use std::{cell::RefCell, rc::Rc, time::Duration};

slint::include_modules!();

type SharedNotes = Rc<RefCell<Vec<Note>>>;
type SharedWindows = Rc<RefCell<Vec<(u64, slint::Weak<NoteWindow>)>>>;

fn persist(notes: &SharedNotes) {
    if let Err(error) = storage::save(&notes.borrow()) {
        eprintln!("NoteBot: could not save notes: {error}");
    }
}

fn open_note(
    note: Note,
    notes: SharedNotes,
    windows: SharedWindows,
) -> Result<(), slint::PlatformError> {
    let window = NoteWindow::new()?;
    let weak_window = window.as_weak();
    let note_id = note.id;

    if let (Some(x), Some(y)) = (note.x, note.y) {
        if x.is_finite() && y.is_finite() {
            window.window().set_position(slint::LogicalPosition::new(
                x.clamp(-100_000.0, 100_000.0),
                y.clamp(-100_000.0, 100_000.0),
            ));
        }
    }
    if let (Some(width), Some(height)) = (note.width, note.height) {
        if width.is_finite() && height.is_finite() {
            window.window().set_size(slint::LogicalSize::new(
                width.clamp(170.0, 8_192.0),
                height.clamp(130.0, 8_192.0),
            ));
        }
    }

    window.set_note_text(note.text.clone().into());
    let color = note
        .custom_color
        .as_deref()
        .and_then(parse_hex_color)
        .unwrap_or(PALETTE[note.color % PALETTE.len()]);
    window.set_paper_color(slint::Color::from_rgb_u8(color[0], color[1], color[2]));
    window
        .set_custom_color_text(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]).into());
    window.set_text_scale(note.text_scale.clamp(0.2, 4.0));

    if !notes.borrow().iter().any(|existing| existing.id == note_id) {
        notes.borrow_mut().push(note.clone());
    }

    {
        let notes = notes.clone();
        let windows = windows.clone();
        let close_window = window.as_weak();
        window.window().on_close_requested(move || {
            if let Some(window) = close_window.upgrade() {
                let (x, y, width, height) = note_geometry(&window);
                if let Some(item) = notes
                    .borrow_mut()
                    .iter_mut()
                    .find(|item| item.id == note_id)
                {
                    item.x = Some(x);
                    item.y = Some(y);
                    item.width = Some(width);
                    item.height = Some(height);
                }
            }
            windows.borrow_mut().retain(|(id, _)| *id != note_id);
            persist(&notes);
            slint::CloseRequestResponse::HideWindow
        });
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
        let windows = windows.clone();
        window.on_delete_note(move || {
            notes.borrow_mut().retain(|item| item.id != note_id);
            windows.borrow_mut().retain(|(id, _)| *id != note_id);
            persist(&notes);
            if let Some(window) = weak_window.upgrade() {
                let _ = window.hide();
            }
        });
    }
    {
        let notes = notes.clone();
        let weak_window = window.as_weak();
        window.on_toggle_color_picker(move || {
            if let Some(window) = weak_window.upgrade() {
                let opening = !window.get_show_color_picker();
                window.set_show_color_picker(opening);
                window.set_color_error("".into());
                if opening {
                    if let Some(item) = notes.borrow().iter().find(|item| item.id == note_id) {
                        let current = item
                            .custom_color
                            .as_deref()
                            .and_then(parse_hex_color)
                            .unwrap_or(PALETTE[item.color % PALETTE.len()]);
                        window.set_custom_color_text(
                            format!("#{:02X}{:02X}{:02X}", current[0], current[1], current[2])
                                .into(),
                        );
                    }
                }
            }
        });
    }
    {
        let notes = notes.clone();
        let weak_window = window.as_weak();
        window.on_select_palette_color(move |index: i32| {
            if let Some(item) = notes
                .borrow_mut()
                .iter_mut()
                .find(|item| item.id == note_id)
            {
                item.color = (index as usize) % PALETTE.len();
                item.custom_color = None;
                let color = PALETTE[item.color];
                if let Some(window) = weak_window.upgrade() {
                    window.set_paper_color(slint::Color::from_rgb_u8(color[0], color[1], color[2]));
                    window.set_show_color_picker(false);
                    window.set_color_error("".into());
                }
                persist(&notes);
            }
        });
    }
    {
        let notes = notes.clone();
        let weak_window = window.as_weak();
        window.on_apply_custom_color(move |value: SharedString| {
            let Some(color) = parse_hex_color(&value) else {
                if let Some(window) = weak_window.upgrade() {
                    window.set_color_error("Enter a hex value such as #80C0FF".into());
                }
                return;
            };
            if let Some(item) = notes
                .borrow_mut()
                .iter_mut()
                .find(|item| item.id == note_id)
            {
                item.custom_color =
                    Some(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]));
                if let Some(window) = weak_window.upgrade() {
                    window.set_paper_color(slint::Color::from_rgb_u8(color[0], color[1], color[2]));
                    window.set_show_color_picker(false);
                    window.set_color_error("".into());
                }
                persist(&notes);
            }
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
    windows.borrow_mut().push((note_id, window.as_weak()));
    persist(&notes);
    Ok(())
}

fn saved_geometry_changed(saved: Option<f32>, current: f32) -> bool {
    saved.map_or(true, |saved| (saved - current).abs() > 0.5)
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

fn note_geometry(window: &NoteWindow) -> (f32, f32, f32, f32) {
    let native_window = window.window();
    let scale = native_window.scale_factor();
    let position = native_window.position().to_logical(scale);
    let size = native_window.size().to_logical(scale);
    (position.x, position.y, size.width, size.height)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = storage::load().unwrap_or_else(|error| {
        eprintln!("NoteBot: could not load saved notes: {error}");
        Vec::new()
    });
    let notes = Rc::new(RefCell::new(loaded));
    let windows = Rc::new(RefCell::new(Vec::new()));
    if notes.borrow().is_empty() {
        notes.borrow_mut().push(Note::new(1));
    }

    for note in notes.borrow().clone() {
        open_note(note, notes.clone(), windows.clone())?;
    }

    let geometry_timer = Timer::default();
    {
        let notes = notes.clone();
        let windows = windows.clone();
        geometry_timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
            let geometry = {
                let mut windows = windows.borrow_mut();
                windows.retain(|(_, weak_window)| weak_window.upgrade().is_some());
                windows
                    .iter()
                    .filter_map(|(id, weak_window)| {
                        weak_window.upgrade().map(|window| {
                            let (x, y, width, height) = note_geometry(&window);
                            (*id, x, y, width, height)
                        })
                    })
                    .collect::<Vec<_>>()
            };

            let changed = {
                let mut notes = notes.borrow_mut();
                let mut changed = false;
                for (id, x, y, width, height) in geometry {
                    if let Some(item) = notes.iter_mut().find(|item| item.id == id) {
                        if saved_geometry_changed(item.x, x)
                            || saved_geometry_changed(item.y, y)
                            || saved_geometry_changed(item.width, width)
                            || saved_geometry_changed(item.height, height)
                        {
                            item.x = Some(x);
                            item.y = Some(y);
                            item.width = Some(width);
                            item.height = Some(height);
                            changed = true;
                        }
                    }
                }
                changed
            };

            if changed {
                persist(&notes);
            }
        });
    }
    persist(&notes);
    slint::run_event_loop()?;
    Ok(())
}
