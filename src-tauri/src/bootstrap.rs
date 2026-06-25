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
