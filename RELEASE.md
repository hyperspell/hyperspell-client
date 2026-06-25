# Go-live checklist

The app is feature-complete for everything that doesn't need Apple credentials.
These are the remaining steps to ship the first signed release. Blocked on an
Apple Developer Program membership + Developer ID Application certificate.

## 1. Secrets (GitHub repo → Settings → Secrets → Actions)

Apple Developer ID signing + notarization:

- [ ] `APPLE_CERTIFICATE` — base64 of the Developer ID Application cert (.p12)
- [ ] `APPLE_CERTIFICATE_PASSWORD`
- [ ] `APPLE_SIGNING_IDENTITY` — e.g. `Developer ID Application: Hyperspell (TEAMID)`
- [ ] `APPLE_ID`
- [ ] `APPLE_PASSWORD` — app-specific password
- [ ] `APPLE_TEAM_ID`

App-shell updater signing (run `npm run tauri signer generate`):

- [ ] `TAURI_SIGNING_PRIVATE_KEY`
- [ ] `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- [ ] Paste the matching **public** key into `src-tauri/tauri.conf.json`
      (`plugins.updater.pubkey`, currently a placeholder)

## 2. Updater hosting

- [ ] Host `latest.json` at the endpoint in `tauri.conf.json`
      (`https://app.hyperspell.com/desktop/latest.json`) — served by the web app
      next to the daemon wheels.

## 3. First signed build

- [ ] Push a `v0.1.0` tag → `.github/workflows/release.yml` runs
      `fetch-runtime.sh` + a signed, notarized `tauri build` (per-arch matrix).
- [ ] **Debug the embedded-Python entitlements** (`src-tauri/entitlements.plist`)
      — the one thing only a real notarized build can validate. Confirm the
      bundled CPython launches and `import httpx` works under hardened runtime +
      library validation. This is the highest-risk item.

## 4. Distribution

- [ ] Swap the dashboard's `curl | bash` install card for a "Download for macOS"
      button (monorepo: `apps/web/app/api/daemon/install.sh/route.ts` + the card).
- [ ] Optional: a Homebrew cask once the signed `.dmg` is published.

## Dependency

The daemon-side consent model + `login --json` + `sync --supervised` must land:
**hyperspell/hyperspell PR #1966**.
