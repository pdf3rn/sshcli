$ErrorActionPreference = "Continue"
$failed = $false

function Run-Step {
    param([scriptblock]$Command, [string]$Label)
    Write-Host ""
    Write-Host ">>> $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        $script:failed = $true
    }
}

$manifest = $env:CARGO_MANIFEST
if (-not $manifest) {
    if (Test-Path "Cargo.toml") {
        $manifest = "Cargo.toml"
    } elseif (Test-Path "src-tauri/Cargo.toml") {
        $manifest = "src-tauri/Cargo.toml"
    }
}

if ((Get-Command cargo -ErrorAction SilentlyContinue) -and $manifest) {
    Run-Step { cargo fmt --manifest-path $manifest --all -- --check } "cargo fmt"
    Run-Step { cargo check --manifest-path $manifest --workspace } "cargo check"
    Run-Step { cargo test --manifest-path $manifest --workspace } "cargo test"
} else {
    Write-Host "Skipping Rust checks: cargo or a Cargo.toml was not found."
    Write-Host "Set CARGO_MANIFEST if the target manifest is elsewhere."
}

if ($env:SLINT_ENTRY) {
    if (Get-Command slint-viewer -ErrorAction SilentlyContinue) {
        Run-Step { slint-viewer --check $env:SLINT_ENTRY } "slint-viewer --check"
    } else {
        Write-Host "SLINT_ENTRY is set but slint-viewer is not installed."
        $failed = $true
    }
} else {
    Write-Host "SLINT_ENTRY not set; standalone slint-viewer check skipped."
}

if ($failed) { exit 1 } else { exit 0 }
