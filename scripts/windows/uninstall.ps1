param(
    [string]$InstallDir = "$HOME\.noo\bin"
)

$ErrorActionPreference = "Stop"

Write-Host "=== nooshell Uninstaller ===" -ForegroundColor Cyan
Write-Host ""

# Remove binary
if (Test-Path "$InstallDir\noo.exe") {
    Remove-Item -LiteralPath "$InstallDir\noo.exe" -Force
    Write-Host "Removed $InstallDir\noo.exe" -ForegroundColor Yellow
}

# Remove install dir if empty
if (Test-Path $InstallDir) {
    $remaining = Get-ChildItem -LiteralPath $InstallDir -ErrorAction SilentlyContinue
    if (!$remaining) {
        Remove-Item -LiteralPath $InstallDir -Force
        Write-Host "Removed empty directory $InstallDir" -ForegroundColor Yellow
        # Also remove parent .noo if empty
        $parentDir = Split-Path -Parent $InstallDir
        $parentRemaining = Get-ChildItem -LiteralPath $parentDir -ErrorAction SilentlyContinue
        if (!$parentRemaining) {
            Remove-Item -LiteralPath $parentDir -Force
            Write-Host "Removed empty directory $parentDir" -ForegroundColor Yellow
        }
    }
}

# Remove from PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -split ";" -contains $InstallDir) {
    $newPath = ($userPath -split ";" | Where-Object { $_ -ne $InstallDir }) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Removed $InstallDir from user PATH" -ForegroundColor Yellow
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")
}

Write-Host ""
Write-Host "=== Done ===" -ForegroundColor Cyan
Write-Host "noo has been uninstalled." -ForegroundColor Gray
