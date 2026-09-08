New-Item -ItemType Directory -Force -Path ".migration" | Out-Null

$files = @{
    ".opencode/templates/state.md" = ".migration/state.md"
    ".opencode/templates/feature-matrix.md" = ".migration/feature-matrix.md"
    ".opencode/templates/screen-inventory.md" = ".migration/screen-inventory.md"
    ".opencode/templates/decision-log.md" = ".migration/decision-log.md"
    ".opencode/templates/blockers.md" = ".migration/blockers.md"
    ".opencode/templates/migration-plan.md" = ".migration/migration-plan.md"
    ".opencode/templates/source-target-classification.md" = ".migration/source-target-classification.md"
}

foreach ($src in $files.Keys) {
    $dst = $files[$src]
    if (-not (Test-Path $dst)) {
        Copy-Item $src $dst
        Write-Host "created $dst"
    } else {
        Write-Host "kept existing $dst"
    }
}
