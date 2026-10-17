//! The sandbox around the one process that talks to the provider: `curl` inside
//! `bwrap`, with the network and nothing else.
//!
//! The credential travels in the curl configuration on stdin, never in an
//! argument: `ps` shows `bwrap … /usr/bin/curl -N -K -` and nothing more.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Files bound read-only because curl cannot resolve a name or verify a
/// certificate without them. Both CA paths are needed: the bundle curl reads is
/// a symlink out of `/etc/ssl` on this machine.
const READ_ONLY_FILES: &[&str] = &[
    "/etc/resolv.conf",
    "/etc/hosts",
    "/etc/nsswitch.conf",
    "/etc/host.conf",
    "/etc/ssl",
    "/etc/ca-certificates",
];

/// `bwrap` cannot exec anything without the loader, and on a merged-`/usr`
/// system the loader is reached through `/lib64`. Verified on this host: with
/// only `/usr` bound, every exec fails with ENOENT.
const SYMLINKS: &[(&str, &str)] = &[
    ("usr/lib", "/lib"),
    ("usr/lib64", "/lib64"),
    ("usr/bin", "/bin"),
    ("usr/sbin", "/sbin"),
];

/// The sandbox's argument list, minus the trailing curl arguments. Pure, so the
/// containment properties can be asserted without running anything.
pub fn argv() -> Vec<OsString> {
    let mut a: Vec<OsString> = vec![
        "bwrap".into(),
        // An empty environment: without this the child inherits the panel's,
        // credential and HOME included (measured with /usr/bin/env inside).
        "--clearenv".into(),
        // Own everything except the network, which is what it needs.
        "--unshare-all".into(),
        "--share-net".into(),
        // No process outlives the panel, and no terminal to inject into.
        "--die-with-parent".into(),
        "--new-session".into(),
        "--ro-bind".into(),
        "/usr".into(),
        "/usr".into(),
    ];
    for (target, link) in SYMLINKS {
        a.push("--symlink".into());
        a.push((*target).into());
        a.push((*link).into());
    }
    for path in READ_ONLY_FILES {
        // Optional: a machine may keep DNS or CA configuration elsewhere, and
        // binding a missing path would make bwrap fail the whole request.
        if std::path::Path::new(path).exists() {
            a.push("--ro-bind".into());
            a.push((*path).into());
            a.push((*path).into());
        }
    }
    a.push("--proc".into());
    a.push("/proc".into());
    a.push("--dev".into());
    a.push("/dev".into());
    a
}

/// `curl` by absolute path: with `--clearenv` there is no `PATH` inside to
/// search, so the path is resolved here.
pub fn curl_path() -> PathBuf {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("curl");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("/usr/bin/curl")
}

/// Start one sandboxed request. The returned child's stdin takes the curl
/// configuration and its stdout is the provider's stream.
pub fn start() -> std::io::Result<Child> {
    let mut args = argv();
    args.push(curl_path().into_os_string());
    // `-N` keeps the stream unbuffered, `-K -` reads the whole configuration
    // (URL, credential, body, timeouts) from stdin.
    args.push("-N".into());
    args.push("-K".into());
    args.push("-".into());
    Command::new(&args[0])
        .args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Whether a failure came from bwrap rather than from the provider: bwrap says
/// so on stderr, and nothing else in the chain runs if it fails.
pub fn is_sandbox_failure(stderr: &str) -> bool {
    stderr.contains("bwrap:")
}

/// Human-readable reason from a failed sandbox start.
pub fn sandbox_error(stderr: &str) -> String {
    let first = stderr
        .lines()
        .find(|l| l.contains("bwrap:"))
        .unwrap_or("bwrap could not start the sandbox")
        .trim();
    format!("provider request not sent: {first}")
}

/// Hand the curl configuration to the child and close stdin, so the child sees
/// end of input and starts the request. Written from this thread because the
/// caller reads stdout on another: a child that is slow to start must not be
/// able to deadlock the reader against a full pipe buffer.
pub fn write_config(child: &mut Child, config: &str) -> std::io::Result<()> {
    let stdin = child.stdin.as_mut().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "no stdin on the sandbox")
    })?;
    stdin.write_all(config.as_bytes())?;
    stdin.flush()?;
    drop(child.stdin.take());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> Vec<String> {
        argv()
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_sandbox_clears_the_environment_and_shares_only_the_network() {
        let a = args();
        assert!(a.contains(&"--clearenv".to_string()));
        assert!(a.contains(&"--share-net".to_string()));
        assert!(a.contains(&"--unshare-all".to_string()));
        assert!(a.contains(&"--die-with-parent".to_string()));
        assert!(a.contains(&"--new-session".to_string()));
    }

    #[test]
    fn the_loader_symlinks_are_present_because_exec_depends_on_them() {
        let a = args();
        for (_, link) in SYMLINKS {
            assert!(a.contains(&(*link).to_string()), "{link} must be linked");
        }
        assert!(a.contains(&"/usr".to_string()));
    }

    #[test]
    fn nothing_in_the_arguments_is_secret() {
        // The whole point: the credential is not an argument, so it is in no
        // process table. Anything added here must keep this true.
        let a = args().join(" ");
        assert!(!a.contains("Bearer"), "no credential material in argv");
        assert!(!a.contains("Authorization"));
        assert!(
            a.contains("-K") || !a.contains("curl"),
            "argv stops before curl"
        );
    }

    #[test]
    fn the_home_directory_and_the_runtime_directory_are_not_bound() {
        let a = args();
        let home = std::env::var("HOME").unwrap_or_default();
        assert!(!a.iter().any(|x| !home.is_empty() && x == &home));
        assert!(
            !a.iter()
                .any(|x| x.contains("XDG") || x.contains("/run/user"))
        );
        assert!(!a.iter().any(|x| x.contains(".config")));
    }

    /// Runs the sandbox on this machine: skipped when there is no bwrap.
    fn sandbox_runs(program: &str, args: &[&str]) -> Option<std::process::Output> {
        let mut cmd = Command::new("bwrap");
        cmd.args(argv().into_iter().skip(1))
            .arg(program)
            .args(args)
            .env("PATH", std::env::var("PATH").unwrap_or_default());
        let out = cmd.output().ok()?;
        Some(out)
    }

    #[test]
    fn the_sandbox_starts_and_runs_a_program() {
        let Some(out) = sandbox_runs("/usr/bin/true", &[]) else {
            return; // no bwrap on this machine
        };
        assert!(
            out.status.success(),
            "bwrap must be able to start a process: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn the_users_home_is_invisible_inside() {
        let home = std::env::var("HOME").unwrap_or("/home".to_string());
        let Some(out) = sandbox_runs("/usr/bin/ls", &["-d", &home]) else {
            return;
        };
        assert!(
            !out.status.success(),
            "the home directory must not exist inside the sandbox: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    #[test]
    fn a_network_request_still_works_from_inside() {
        // DNS and TLS need the bound files; without them the sandbox is useless.
        let Some(out) = sandbox_runs(
            "/usr/bin/curl",
            &[
                "-sS",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "https://api.commandcode.ai/provider/v1/models",
            ],
        ) else {
            return;
        };
        let code = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            code.trim(),
            "200",
            "a sandboxed request must resolve DNS and verify TLS: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn bwrap_failures_are_recognised() {
        assert!(is_sandbox_failure(
            "bwrap: setting up uid map: Permission denied"
        ));
        assert!(!is_sandbox_failure("curl: (6) Could not resolve host"));
        assert!(
            sandbox_error("bwrap: No permissions to creating new namespace")
                .starts_with("provider request not sent:")
        );
    }
}

#[cfg(test)]
mod slow_child_tests {
    use super::*;

    /// The classic two-pipe deadlock needs the child to block writing stdout
    /// while we block writing stdin. Ours cannot: the configuration is written
    /// before any stdout is consumed, and stdout is drained by the feed thread.
    /// A sandboxed `cat` is the slowest child we can ask for — bwrap must build
    /// the namespaces before it reads a byte.
    #[test]
    fn a_large_configuration_survives_a_slow_child() {
        let Ok(mut child) = std::process::Command::new("bwrap")
            .args(argv().into_iter().skip(1))
            .arg("/usr/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        else {
            return; // no bwrap on this machine
        };
        let config = "x".repeat(100 * 1024);
        write_config(&mut child, &config).expect("write");
        let mut out = String::new();
        std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut out).expect("read");
        assert_eq!(out.len(), config.len(), "the whole configuration arrived");
        assert!(child.wait().expect("wait").success());
    }
}
