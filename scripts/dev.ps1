#Requires -Version 5.1
<#
.SYNOPSIS
  Start 神人网 frontend (Vite HMR) + backend (watchexec → cargo run) together.

.NOTES
  Prerequisites:
    - Rust toolchain + cargo
    - Node.js + npm
    - watchexec: cargo install watchexec-cli
    - Root deps: npm install (concurrently)
    - Frontend deps: npm --prefix frontend install (when frontend exists)
#>

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Test-Command($Name) {
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

if (-not (Test-Command "cargo")) {
    Write-Error "cargo not found. Install Rust from https://rustup.rs/"
}
if (-not (Test-Command "watchexec")) {
    Write-Error "watchexec not found. Run: cargo install watchexec-cli"
}
if (-not (Test-Command "npm")) {
    Write-Error "npm not found. Install Node.js."
}

if (-not (Test-Path (Join-Path $Root "node_modules"))) {
    Write-Host "Installing root npm deps (concurrently)..."
    npm install
}

$frontendPkg = Join-Path $Root "frontend\package.json"
if (Test-Path $frontendPkg) {
    if (-not (Test-Path (Join-Path $Root "frontend\node_modules"))) {
        Write-Host "Installing frontend deps..."
        npm --prefix frontend install
    }
} else {
    Write-Warning "frontend/ not found yet — starting backend only. Frontend may be scaffolded separately."
}

$env:DATABASE_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "sqlite://data/shenren.db?mode=rwc" }
$env:BIND_ADDR = if ($env:BIND_ADDR) { $env:BIND_ADDR } else { "127.0.0.1:3000" }

Write-Host "DATABASE_URL=$($env:DATABASE_URL)"
Write-Host "BIND_ADDR=$($env:BIND_ADDR)"
Write-Host "Backend hot reload: watchexec -e rs,toml -r cargo run (cwd backend/)"

if (Test-Path $frontendPkg) {
    npm run dev
} else {
    Set-Location (Join-Path $Root "backend")
    watchexec -e rs,toml -r cargo run
}
