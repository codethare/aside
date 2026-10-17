//! The one way text leaves the panel: `wl-copy`, on the highlighted line, when
//! the user asks for it.
//!
//! `wl-copy` keeps a process alive to serve the selection, so the panel does not
//! wait for it: it hands over the line and lets the child serve the clipboard on
//! its own. That process is the single thing that outlives the panel, and it
//! holds only what the user chose to copy.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Copy `line` to the clipboard. Returns as soon as the line is handed over.
pub fn copy(line: &str) -> Result<(), String> {
    copy_with(Path::new("wl-copy"), line)
}

/// The testable core: any program that reads the line on stdin.
pub fn copy_with(program: &Path, line: &str) -> Result<(), String> {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "wl-copy is not installed: install wl-clipboard to copy".to_string()
            }
            _ => format!("could not start wl-copy: {e}"),
        })?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| format!("could not write to wl-copy: {e}"))?;
    }
    drop(child.stdin.take());
    // Reaped on a thread: wl-copy forks to serve the selection, and the panel
    // must not wait for that.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A stub that records what it was handed, so the test can assert the panel
    /// copies one line and only one line.
    fn stub(dir: &Path, log: &Path, separate: bool) -> PathBuf {
        let path = dir.join("wl-copy");
        let path_str = log.to_str().expect("utf-8 path");
        let script = if separate {
            // A separator makes concurrent writes from two stubs separable.
            format!("#!/bin/sh\ncat >> '{path_str}'\nprintf '\\n' >> '{path_str}'\n")
        } else {
            format!("#!/bin/sh\ncat >> '{path_str}'\n")
        };
        std::fs::write(&path, script).expect("write stub");
        let mut perms = std::fs::metadata(&path).expect("stat").permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod");
        path
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aside-copy-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    #[test]
    fn the_highlighted_line_reaches_the_clipboard_and_nothing_else_does() {
        let dir = scratch("line");
        let log = dir.join("log");
        let program = stub(&dir, &log, false);
        copy_with(&program, "just this line").expect("copies");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let read = || std::fs::read_to_string(&log).unwrap_or_default();
        while read() != "just this line" && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            read(),
            "just this line",
            "exactly the line, with no newline or transcript around it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_press_is_one_process() {
        let dir = scratch("twice");
        let log = dir.join("log");
        let program = stub(&dir, &log, true);
        copy_with(&program, "one").expect("copies");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let lines = || {
            std::fs::read_to_string(&log)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        while lines().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // The second press waits for the first stub to exit: a shell script
        // cannot be executed twice at once, which is a property of the test's
        // stub rather than of wl-copy.
        copy_with(&program, "two").expect("copies");
        while lines().len() < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Two presses, two processes, one line each.
        let mut got = lines();
        got.sort();
        assert_eq!(got, ["one", "two"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_wl_copy_says_what_to_install() {
        let err = copy_with(Path::new("/nonexistent/wl-copy"), "x").expect_err("fails");
        assert!(err.contains("wl-clipboard"), "{err}");
    }
}
