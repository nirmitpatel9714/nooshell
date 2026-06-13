$ProjectRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSCommandPath))
$BinDir = "$env:USERPROFILE\.noo\bin"
$Binary = "$BinDir\noo.exe"

Write-Host "==> Building noobook (release)..." -ForegroundColor Cyan
cargo build --release --manifest-path "$ProjectRoot\Cargo.toml"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Installing binary to $BinDir" -ForegroundColor Cyan
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
Copy-Item "$ProjectRoot\target\release\noo.exe" $Binary -Force

$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$BinDir*") {
  [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$BinDir", "User")
  Write-Host "==> Added $BinDir to user PATH" -ForegroundColor Cyan
  Write-Host "==> Restart your terminal or run: `$env:Path += `";$BinDir`"" -ForegroundColor Yellow
}

Write-Host "==> noo installed successfully" -ForegroundColor Green
