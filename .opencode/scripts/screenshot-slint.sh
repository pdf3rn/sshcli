#!/usr/bin/env bash
set -euo pipefail

if [ -z "${SLINT_ENTRY:-}" ]; then
  echo "Set SLINT_ENTRY to a previewable .slint file."
  exit 2
fi

if ! command -v slint-viewer >/dev/null 2>&1; then
  echo "slint-viewer not found. Install with: cargo install slint-viewer"
  exit 2
fi

out="${1:-.migration/screenshots/slint-current.png}"
mkdir -p "$(dirname "$out")"

slint-viewer --check "$SLINT_ENTRY"
slint-viewer --screenshot "$out" "$SLINT_ENTRY"

echo "$out"
