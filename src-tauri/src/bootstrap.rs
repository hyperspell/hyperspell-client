//! First-run bootstrap: materialize the daemon's venv from the runtime bundled
//! inside the app (a relocatable CPython + `uv` + the daemon wheel).
//!
//! Option A: the venv is built at `~/.hyperspell/venv` — OUTSIDE the signed .app
//! bundle — so the daemon's own SHA256-verified wheel self-update (`uv pip
//! install` into that venv) keeps working without invalidating the app's code
//! signature.

use crate::daemon_paths::hyperspell_dir;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bump to force a venv rebuild after an app update ships a newer runtime/wheel.
const RUNTIME_VERSION: &str = "1";

pub struct Runtime {
    pub uv: PathBuf,
    pub python: PathBuf,
    pub wheel: PathBuf,
}

/// Resolve the bundled runtime from the app's resource dir. Layout matches
/// `scripts/fetch-runtime.sh` + the `resources` map in tauri.conf.json.
pub fn runtime_from_resources(resource_dir: &Path) -> Runtime {
    let base = resource_dir.join("runtime");
    Runtime {
        uv: base.join("uv"),
        python: base.join("python").join("bin").join("python3"),
        wheel: find_wheel(&base),
    }
}

fn find_wheel(base: &Path) -> PathBuf {
    // The wheel filename carries a version; pick the single *.whl present.
    std::fs::read_dir(base)
        .ok()
        .and_then(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .find(|p| p.extension().is_some_and(|x| x == "whl"))
        })
        .unwrap_or_else(|| base.join("hyperspell.whl"))
}

/// True when a usable bundled runtime is present (false in `tauri dev`, where the
/// app falls back to a daemon already installed on PATH / in ~/.hyperspell/venv).
pub fn is_bundled(rt: &Runtime) -> bool {
    rt.uv.is_file() && rt.python.is_file() && rt.wheel.is_file()
}

/// Ensure `~/.hyperspell/venv` exists with the bundled daemon installed. A cheap
/// no-op once built for this runtime version. Returns the daemon binary path.
///
/// TODO(ux): the first build can take a few seconds and the wheel's deps
/// (httpx/click) are resolved from PyPI on first run — emit progress events so
/// the UI can show a "setting up…" state instead of blocking.
pub fn ensure_venv(rt: &Runtime) -> Result<PathBuf, String> {
    let hs = hyperspell_dir();
    let venv = hs.join("venv");
    let daemon = venv.join("bin").join("hyperspell");
    let marker = hs.join(".runtime-version");

    let current = std::fs::read_to_string(&marker).unwrap_or_default();
    if daemon.is_file() && current.trim() == RUNTIME_VERSION {
        return Ok(daemon); // already built for this app version
    }

    std::fs::create_dir_all(&hs).map_err(|e| e.to_string())?;

    // Build the venv with the bundled interpreter, then install the bundled wheel.
    run(
        &rt.uv,
        &[
            "venv",
            "--python",
            &rt.python.to_string_lossy(),
            &venv.to_string_lossy(),
        ],
    )?;
    run(
        &rt.uv,
        &[
            "pip",
            "install",
            "--python",
            &venv.join("bin").join("python").to_string_lossy(),
            "--no-cache-dir",
            &rt.wheel.to_string_lossy(),
        ],
    )?;

    std::fs::write(&marker, RUNTIME_VERSION).map_err(|e| e.to_string())?;
    Ok(daemon)
}

/// The full `hyperbrain` query CLI (ask/search/remember/memories/connections/
/// integrations/brain/structure/…) + its MCP server, pinned to the same minor
/// the daemon manages.
const HYPERBRAIN_DIST: &str = "hyperspell-brain[mcp]>=0.4,<0.5";

/// Install the complete `hyperbrain` CLI via the bundled uv so every brain
/// ability is available right after install (not only after the daemon's first
/// sync, which also does this). `uv tool install` drops `hyperbrain` +
/// `hyperbrain-mcp` into ~/.local/bin. Best-effort: needs PyPI on first run, so
/// the caller treats failure as non-fatal (the daemon retries on its next tick).
pub fn install_hyperbrain(rt: &Runtime) -> Result<(), String> {
    run(&rt.uv, &["tool", "install", "--force", HYPERBRAIN_DIST])
}

/// Expose the CLIs on the user's PATH — the actual "trusted install" deliverable.
/// After the app runs once, `hyperspell` works from any terminal, and `uv` is
/// available so the daemon can install/upgrade the `hyperbrain` CLI and run its
/// own wheel self-update. Mirrors what the old curl|bash installer did
/// (~/.local/bin symlink + a shell-rc PATH line), best-effort and idempotent.
pub fn expose_cli(rt: &Runtime, daemon: &Path) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("no home dir")?;
    let local_bin = home.join(".local").join("bin");
    std::fs::create_dir_all(&local_bin).map_err(|e| e.to_string())?;

    // Copy uv to a stable location OUTSIDE the .app bundle so the link survives
    // app moves/updates (the bundled copy under Resources would dangle).
    let uv_stable = hyperspell_dir().join("bin").join("uv");
    if let Some(parent) = uv_stable.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&rt.uv, &uv_stable).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&uv_stable, std::fs::Permissions::from_mode(0o755));
    }

    symlink_force(daemon, &local_bin.join("hyperspell"))?;
    symlink_force(&uv_stable, &local_bin.join("uv"))?;
    ensure_on_path(&local_bin);
    Ok(())
}

#[cfg(unix)]
fn symlink_force(target: &Path, link: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(link); // replace a stale link/file
    std::os::unix::fs::symlink(target, link).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn symlink_force(_target: &Path, _link: &Path) -> Result<(), String> {
    Ok(()) // TODO(windows): PATH exposure when Windows support lands
}

/// Append a `~/.local/bin` PATH line to the user's shell rc files if absent.
/// Best-effort: a missing rc or write error is not fatal — the symlinks still
/// exist for shells that already include ~/.local/bin.
fn ensure_on_path(local_bin: &Path) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let line = "export PATH=\"$HOME/.local/bin:$PATH\"  # hyperspell";
    for rc in [".zshrc", ".bashrc"] {
        let path = home.join(rc);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if existing.contains(&*local_bin.to_string_lossy()) || existing.contains(line) {
            continue;
        }
        let body = if existing.is_empty() {
            format!("{line}\n")
        } else {
            format!("{}\n{line}\n", existing.trim_end())
        };
        let _ = std::fs::write(&path, body);
    }
}

fn run(bin: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} failed: {}",
            bin.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}
