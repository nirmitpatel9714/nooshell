$BinDir = "$env:USERPROFILE\.noo\bin"
$Binary = "$BinDir\noo.exe"

if (Test-Path $Binary) {
  Write-Host "==> Removing binary: $Binary" -ForegroundColor Cyan
  Remove-Item $Binary -Force
}

if (Test-Path $BinDir) {
  $remaining = Get-ChildItem $BinDir -Force
  if (-not $remaining) {
    Remove-Item $BinDir -Force
  }
}

if (Test-Path "$env:USERPROFILE\.noo") {
  $remaining = Get-ChildItem "$env:USERPROFILE\.noo" -Force
  if (-not $remaining) {
    Remove-Item "$env:USERPROFILE\.noo" -Force
  }
}

$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -like "*$BinDir*") {
  $NewPath = ($CurrentPath.Split(';') | Where-Object { $_ -ne $BinDir }) -join ';'
  [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
  Write-Host "==> Removed $BinDir from user PATH" -ForegroundColor Cyan
}

Write-Host "==> noo uninstalled" -ForegroundColor Green
