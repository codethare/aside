//! The panel's two ways across the clipboard: `wl-copy` takes the highlighted line
//! out, `wl-paste` brings the selection in when the user asks for it.
//!
//! `wl-copy` keeps a process alive to serve the selection, so the panel does not
//! wait for it: it hands over the line and lets the child serve the clipboard on
//! its own. That process is the single thing that outlives the panel, and it
//! holds only what the user chose to copy.
//!
//! Reading is never watched: `paste` runs `wl-paste` once, when the key is pressed,
//! and keeps no copy of what came back beyond the input line it was inserted into.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Copy `line` to the clipboard. Returns as soon as the line is handed over.
pub fn copy(line: &str) -> Result<(), String> {
    copy_with(Path::new("wl-copy"), line)
}

/// The clipboard's text, read on the spot. `--no-newline` because the input is one
/// logical line: a trailing newline pasted into it would be invisible and then
/// sent to the provider.
pub fn paste() -> Result<String, String> {
    paste_with(Path::new("wl-paste"))
}

/// The testable core: any program that writes the selection to stdout.
pub fn paste_with(program: &Path) -> Result<String, String> {
    let out = Command::new(program)
        .arg("--no-newline")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "wl-paste is not installed: install wl-clipboard to paste".to_string()
            }
            _ => format!("could not read the clipboard: {e}"),
        })?;
    if !out.status.success() {
        return Err(format!("the clipboard could not be read ({})", out.status));
    }
    // Newlines inside the selection would break the one logical line the panel has;
    // they become spaces, which is what the question would have been typed as.
    Ok(String::from_utf8_lossy(&out.stdout).replace(['\r', '\n'], " "))
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

    /// `wl-paste --no-newline` is read once, on the key: the newline flag keeps the
    /// input one logical line, and a selection with newlines in it becomes spaces.
    #[test]
    fn the_clipboard_is_read_once_when_asked() {
        let dir = scratch("paste");
        let path = dir.join("wl-paste");
        std::fs::write(
            &path,
            "#!/bin/sh\n[ \"$1\" = \"--no-newline\" ] || exit 3\nprintf 'one\\ntwo'\n",
        )
        .expect("write stub");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        assert_eq!(
            paste_with(&path).expect("reads"),
            "one two",
            "the flag is passed and the newlines do not escape the one line"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failing_paste_is_reported_rather_than_swallowed() {
        let dir = scratch("paste-fail");
        let path = dir.join("wl-paste");
        std::fs::write(&path, "#!/bin/sh\nexit 1\n").expect("write stub");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let err = paste_with(&path).expect_err("a failed read is an error");
        assert!(err.contains("could not be read"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
        let err = paste_with(Path::new("/nonexistent/wl-paste")).expect_err("fails");
        assert!(err.contains("wl-clipboard"), "{err}");
    }

    #[test]
    fn a_missing_wl_copy_says_what_to_install() {
        let err = copy_with(Path::new("/nonexistent/wl-copy"), "x").expect_err("fails");
        assert!(err.contains("wl-clipboard"), "{err}");
    }
}
