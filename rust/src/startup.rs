/*
 * NoteBot Rust port: copyright is held by its respective contributors.
 * This file is distributed under GNU GPL version 3 only.
 */
use std::io;

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "NoteBot";

pub fn install() -> io::Result<()> {
    #[cfg(windows)]
    {
        let executable = std::env::current_exe()?;
        let command = format!("\"{}\" --autostartup", executable.display());
        run_registry_command([
            "add", RUN_KEY, "/v", VALUE_NAME, "/t", "REG_SZ", "/d", &command, "/f",
        ])
    }

    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub fn uninstall() -> io::Result<()> {
    #[cfg(windows)]
    {
        run_registry_command(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])
    }

    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

#[cfg(windows)]
fn run_registry_command<const N: usize>(arguments: [&str; N]) -> io::Result<()> {
    let output = Command::new("reg.exe").args(arguments).output()?;
    if output.status.success() {
        Ok(())
    } else {
        let details = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )
        .trim()
        .to_owned();
        Err(io::Error::other(if details.is_empty() {
            "Windows registry command failed".to_owned()
        } else {
            details
        }))
    }
}

#[cfg(not(windows))]
fn unsupported_platform() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows startup registration is only available on Windows",
    )
}
