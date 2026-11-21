# Setup environment PATH for MinGW and Cargo
$env:PATH = "C:\Users\shaur\w64devkit\bin;C:\Users\shaur\.cargo\bin;" + $env:PATH

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "       NeoResonance - Windows 11 Desktop Overlay Widget   " -ForegroundColor Magenta
Write-Host "==========================================================" -ForegroundColor Cyan

$exePath = "NeoResonance.exe"
if (!(Test-Path $exePath)) {
    $exePath = "src-tauri\target\release\neo-resonance.exe"
}
if (!(Test-Path $exePath)) {
    $exePath = "src-tauri\target\debug\neo-resonance.exe"
}

if (!(Test-Path $exePath)) {
    Write-Host "Building NeoResonance production release..." -ForegroundColor Yellow
    npm run build
    cd src-tauri
    cargo build --release
    cd ..
    $exePath = "src-tauri\target\release\neo-resonance.exe"
}

Write-Host "Launching NeoResonance ($exePath)..." -ForegroundColor Green
Start-Process $exePath
