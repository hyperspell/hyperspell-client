# Hyperspell Desktop Client

The Hyperspell menu-bar app for macOS. It keeps your company brain in sync on
your machine and makes it queryable from local AI tools (Claude Code, Codex,
Cursor, Claude Desktop) — with an **explicit consent screen** for everything it
touches, and a signed, notarized installer instead of `curl | bash`.

This repo is intentionally **public and inspectable**: you can audit exactly what
the software running on your machine does. That auditability is the point.

## How it works (wrap, don't rewrite)

The app does **not** reimplement sync. It wraps and supervises the existing,
battle-tested Python sync daemon (`tools/hyperspell-sync` in the private
`hyperspell/hyperspell` monorepo), which is published as a wheel.

- **Supervisor** (`src-tauri/src/supervisor.rs`) — spawns and monitors
  `hyperspell sync --supervised` as a child process. `--supervised` makes the
  daemon tear down any legacy launchd/systemd/cron auto-start on startup, so the
  app is the sole supervisor.
- **Auth** (`src-tauri/src/auth.rs`) — drives `hyperspell login --json`, which
  streams the device-auth flow as newline-delimited JSON; the app renders the
  device code, opens the browser, and waits for the `approved` event.
- **Consent** (`src-tauri/src/permissions.rs`) — writes the daemon's flat
  `perm_*` keys into `~/.hyperspell/config.toml`. The daemon's
  `reconcile_agent_configs()` enforces them: granted integrations are written,
  revoked ones are stripped on the next sync tick. **Default-deny** — nothing is
  written until you allow it.
- **Activity log** (`src-tauri/src/actions_log.rs`) — surfaces the daemon's
  `~/.hyperspell/.actions.jsonl` so every machine-touching action is visible.

### Daemon contract / version coupling

The app depends on a daemon version that includes `login --json`,
`sync --supervised`, and the `perm_*` consent model. It bundles a **released
daemon wheel** (not source), so the two ship on independent release trains — the
app pins "daemon wheel ≥ the version with these features," the same pin pattern
the daemon already uses for the `hyperbrain` CLI.

## Bundling (Option A)

The app ships a relocatable CPython + the daemon wheel, but builds/keeps the
venv at `~/.hyperspell/venv` — **outside** the signed `.app` bundle. This is
deliberate: the daemon's existing SHA256-verified wheel self-update (`uv pip
install` into that venv) keeps working without invalidating the app's code
signature. See `src-tauri/entitlements.plist` for the hardened-runtime
entitlements the embedded interpreter needs.

> **Status:** scaffold. The Rust modules are stubs that map to the design above;
> `daemon_paths::daemon_bin()` currently resolves `hyperspell` on `PATH` (dev
> install) rather than the bundled interpreter. Signing/notarization and the
> bundled-interpreter wiring are not done yet.

## Develop

Prerequisites: **Rust** (`rustup`), **Node 20+**, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for macOS.

```bash
npm install
npm run tauri dev      # run the app
npm run tauri build    # build a (currently unsigned) .app + .dmg
```

The updater public key in `src-tauri/tauri.conf.json` is a placeholder —
generate a real keypair with `npm run tauri signer generate` before shipping
updates.

## Releasing (signed + notarized)

`.github/workflows/release.yml` builds, signs, notarizes, and publishes a draft
GitHub Release (signed `.dmg` + Tauri updater artifacts) on a `v*` tag. It needs
these repo secrets:

**Apple Developer ID signing + notarization** (requires an Apple Developer
Program membership):

| Secret | What it is |
|---|---|
| `APPLE_CERTIFICATE` | Base64 of the Developer ID Application cert (.p12) |
| `APPLE_CERTIFICATE_PASSWORD` | Password for that .p12 |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Hyperspell (TEAMID)` |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_PASSWORD` | App-specific password for that Apple ID |
| `APPLE_TEAM_ID` | 10-char Apple Team ID |

**App-shell updater signing** (independent of Apple — signs the auto-update
payload). Generate once with `npm run tauri signer generate`:

| Secret | What it is |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The generated private key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password |

Put the matching **public** key in `src-tauri/tauri.conf.json`
(`plugins.updater.pubkey`) — it's a placeholder today. Host the updater's
`latest.json` at the endpoint configured there.

## License

MIT — see [LICENSE](LICENSE).
