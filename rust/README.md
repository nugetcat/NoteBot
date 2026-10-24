# NoteBot Rust + Slint

This directory contains the new Rust implementation of NoteBot. The Java/NetBeans version remains in `../StickyNotes` as the reference implementation during the port.

## Status

The current application slice supports multiple borderless, draggable, and resizable note windows, local JSON persistence including each note's position, size, and custom hex color, the original eight note colors, text editing, and text scaling. On first launch, the app imports notes from the Java `sticky.dat` file (or its backups) when no Rust notes file exists, and leaves the source files untouched. Closing a window preserves its note for the next launch; the × button deletes it. Windows sign-in startup can be enabled or removed from the command line; exact desktop layering and taskbar behavior remain to be verified. Existing JSON files without geometry or custom-color fields remain compatible. Custom colors accept six-digit RGB hex values such as `#80C0FF`.

## Build

Install Rust and the native build prerequisites for Slint, then run:

```powershell
cargo run --manifest-path rust/Cargo.toml
```

On Windows the saved notes are stored in the user's local application data directory under `NoteBot/notes.json`; the legacy Java file is in that directory as `sticky.dat`.

On Windows, run cargo run --manifest-path rust/Cargo.toml -- --install-startup to enable startup at sign-in, or cargo run --manifest-path rust/Cargo.toml -- --remove-startup to disable it. These commands update the current user Run registry entry.

## License and attribution

The Rust port uses GNU GPL version 3 only (`GPL-3.0-only`), which is compatible with the original project's GPL-3.0-or-later terms and Slint's GPLv3 option. The legacy Java files retain their existing license notices. The original NoteBot code and visual behavior are credited to Federico Dossena; ported portions will retain attribution as they are migrated. See the `LICENSE` file in this directory and preserve third-party notices for bundled assets and dependencies. GPL protects user freedoms; it does not prohibit charging for copies.
