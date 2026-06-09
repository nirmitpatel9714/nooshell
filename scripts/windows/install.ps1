param(
    [switch]$Release = $true,
    [string]$InstallDir = "$HOME\.noo\bin",
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"

$BinName = "noo.exe"
$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition

Write-Host "=== nooshell Installer ===" -ForegroundColor Cyan
Write-Host ""

# --- Check Rust toolchain ---
if (!(Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Error "Rust/Cargo not found. Install from https://rustup.rs and try again."
    exit 1
}

# --- Build ---
Write-Host "Building nooshell ($(if ($Release) { 'release' } else { 'debug' }))..." -ForegroundColor Yellow
$buildFlag = if ($Release) { "--release" } else { "" }
& cargo build $buildFlag --manifest-path "$RepoRoot\Cargo.toml"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed."
    exit 1
}

$sourceBin = if ($Release) { "$RepoRoot\target\release\$BinName" } else { "$RepoRoot\target\debug\$BinName" }
if (!(Test-Path $sourceBin)) {
    Write-Error "Binary not found at $sourceBin"
    exit 1
}

# --- Install directory ---
Write-Host "Installing to $InstallDir ..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

Copy-Item -LiteralPath $sourceBin -Destination "$InstallDir\$BinName" -Force

Write-Host "Installed $BinName to $InstallDir" -ForegroundColor Green

# --- PATH ---
if (!$NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -split ";" -notcontains $InstallDir) {
        $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Added $InstallDir to user PATH" -ForegroundColor Green
        # Refresh for current session
        $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")
    } else {
        Write-Host "$InstallDir is already on PATH" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "=== Done ===" -ForegroundColor Cyan
Write-Host "Restart your terminal, or run: `$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')" -ForegroundColor Gray
Write-Host "Then run: noo --help" -ForegroundColor Cyan
