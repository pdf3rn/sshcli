#!/usr/bin/env bash
set -uo pipefail

fail=0

run() {
  echo
  echo ">>> $*"
  "$@" || fail=1
}

manifest="${CARGO_MANIFEST:-}"
if [ -z "$manifest" ]; then
  if [ -f Cargo.toml ]; then
    manifest="Cargo.toml"
  elif [ -f src-tauri/Cargo.toml ]; then
    manifest="src-tauri/Cargo.toml"
  fi
fi

if command -v cargo >/dev/null 2>&1 && [ -n "$manifest" ]; then
  run cargo fmt --manifest-path "$manifest" --all -- --check
  run cargo check --manifest-path "$manifest" --workspace
  run cargo test --manifest-path "$manifest" --workspace
else
  echo "Skipping Rust checks: cargo or a Cargo.toml was not found."
  echo "Set CARGO_MANIFEST if the target manifest is elsewhere."
fi

if [ -n "${SLINT_ENTRY:-}" ]; then
  if command -v slint-viewer >/dev/null 2>&1; then
    run slint-viewer --check "$SLINT_ENTRY"
  else
    echo "SLINT_ENTRY is set but slint-viewer is not installed."
    fail=1
  fi
else
  echo "SLINT_ENTRY not set; standalone slint-viewer check skipped."
fi

exit "$fail"
