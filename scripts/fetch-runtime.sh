#!/usr/bin/env bash
# Fetch the runtime bundled inside the app: a relocatable CPython, the `uv`
# package manager, and the daemon wheel. Output lands in
# src-tauri/resources/runtime/, which tauri.conf.json maps into the .app's
# Resources and bootstrap.rs uses on first run to build ~/.hyperspell/venv.
#
# Run before `npm run tauri build`. The CI release workflow runs this too.
#
# Pinned versions — bump deliberately (then bump RUNTIME_VERSION in bootstrap.rs
# so installed apps rebuild their venv).
set -euo pipefail

PYTHON_VERSION="3.12.8"
# python-build-standalone release tag (https://github.com/astral-sh/python-build-standalone/releases)
PBS_TAG="20241219"
UV_VERSION="0.5.11"
# The daemon wheel is hosted on the Vercel CDN by the monorepo's release flow.
WHEEL_URL="${HYPERSPELL_WHEEL_URL:-https://app.hyperspell.com/hyperspell-latest-py3-none-any.whl}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/src-tauri/resources/runtime"
mkdir -p "$OUT"

# macOS arch: build per-arch; for a universal app run this for each and merge, or
# rely on the per-arch CI matrix. Here we fetch for the host arch.
case "$(uname -m)" in
  arm64) PBS_ARCH="aarch64-apple-darwin"; UV_ARCH="aarch64-apple-darwin" ;;
  x86_64) PBS_ARCH="x86_64-apple-darwin"; UV_ARCH="x86_64-apple-darwin" ;;
  *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;;
esac

echo "==> CPython ${PYTHON_VERSION} (${PBS_ARCH})"
PBS_URL="https://github.com/astral-sh/python-build-standalone/releases/download/${PBS_TAG}/cpython-${PYTHON_VERSION}+${PBS_TAG}-${PBS_ARCH}-install_only.tar.gz"
curl -fsSL "$PBS_URL" -o /tmp/cpython.tar.gz
rm -rf "$OUT/python"
mkdir -p "$OUT/python"
# install_only tarballs extract to a top-level `python/` dir.
tar -xzf /tmp/cpython.tar.gz -C "$OUT" python
test -x "$OUT/python/bin/python3" || { echo "python3 not where expected" >&2; exit 1; }

echo "==> uv ${UV_VERSION} (${UV_ARCH})"
UV_URL="https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/uv-${UV_ARCH}.tar.gz"
curl -fsSL "$UV_URL" -o /tmp/uv.tar.gz
tar -xzf /tmp/uv.tar.gz -C /tmp
cp "/tmp/uv-${UV_ARCH}/uv" "$OUT/uv"
chmod +x "$OUT/uv"

echo "==> daemon wheel"
rm -f "$OUT"/*.whl
curl -fsSL "$WHEEL_URL" -o "$OUT/hyperspell.whl"

echo "==> done:"
ls -la "$OUT"
