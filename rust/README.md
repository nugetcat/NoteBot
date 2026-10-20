# NoteBot Rust + Slint

This directory contains the new Rust implementation of NoteBot. The Java/NetBeans version remains in `../StickyNotes` as the reference implementation during the port.

## Status

The current application slice supports multiple borderless, draggable, and resizable note windows, local JSON persistence including each note's position, size, and custom hex color, the original eight note colors, text editing, and text scaling. Closing a window preserves its note for the next launch; the × button deletes it. The Java `sticky.dat` importer, startup integration, and exact desktop layering/taskbar behavior remain to be ported. Existing JSON files without geometry or custom-color fields remain compatible. Custom colors accept six-digit RGB hex values such as `#80C0FF`.

## Build

Install Rust and the native build prerequisites for Slint, then run:

```powershell
cargo run --manifest-path rust/Cargo.toml
```

On Windows the saved notes are stored in the user's local application data directory under `NoteBot/notes.json`. This deliberately leaves the original Java `sticky.dat` file untouched until a migration importer is implemented.

## License and attribution

The Rust port uses GNU GPL version 3 only (`GPL-3.0-only`), which is compatible with the original project's GPL-3.0-or-later terms and Slint's GPLv3 option. The legacy Java files retain their existing license notices. The original NoteBot code and visual behavior are credited to Federico Dossena; ported portions will retain attribution as they are migrated. See the `LICENSE` file in this directory and preserve third-party notices for bundled assets and dependencies. GPL protects user freedoms; it does not prohibit charging for copies.
