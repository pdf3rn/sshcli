#!/usr/bin/env bash
set -euo pipefail

mkdir -p .migration

copy_if_missing() {
  src="$1"
  dst="$2"
  if [ ! -e "$dst" ]; then
    cp "$src" "$dst"
    echo "created $dst"
  else
    echo "kept existing $dst"
  fi
}

copy_if_missing .opencode/templates/state.md .migration/state.md
copy_if_missing .opencode/templates/feature-matrix.md .migration/feature-matrix.md
copy_if_missing .opencode/templates/screen-inventory.md .migration/screen-inventory.md
copy_if_missing .opencode/templates/decision-log.md .migration/decision-log.md
copy_if_missing .opencode/templates/blockers.md .migration/blockers.md
copy_if_missing .opencode/templates/migration-plan.md .migration/migration-plan.md

copy_if_missing .opencode/templates/source-target-classification.md .migration/source-target-classification.md
