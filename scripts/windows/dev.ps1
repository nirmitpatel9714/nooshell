param(
    [switch]$Release
)

$projectRoot = Resolve-Path "$PSScriptRoot\..\.."
$target = if ($Release) { "release" } else { "debug" }
$binary = "$projectRoot\target\$target\noo.exe"

Write-Host "━━ Compiling nooshell ━━━━" -ForegroundColor Cyan
if ($Release) {
    cargo build --release
} else {
    cargo build
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed." -ForegroundColor Red
    exit 1
}

Write-Host "Build successful." -ForegroundColor Green
Write-Host ""
Write-Host "Arguments passed: $args" -ForegroundColor DarkGray
$choice = Read-Host "Run noo? (Y/n)"
if ($choice -eq "" -or $choice -eq "y" -or $choice -eq "Y") {
    Clear-Host
    & $binary @args
}
