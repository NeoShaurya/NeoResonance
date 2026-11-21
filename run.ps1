# Setup environment PATH for MinGW and Cargo
$env:PATH = "C:\Users\shaur\w64devkit\bin;C:\Users\shaur\.cargo\bin;" + $env:PATH

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "       NeoResonance - Windows 11 Desktop Overlay Widget   " -ForegroundColor Magenta
Write-Host "==========================================================" -ForegroundColor Cyan

# Check if target binary exists, if not build it
if (!(Test-Path "src-tauri\target\debug\neo-resonance.exe")) {
    Write-Host "Building NeoResonance..." -ForegroundColor Yellow
    npm run build
    cd src-tauri
    cargo build
    cd ..
}

Write-Host "Starting NeoResonance on Desktop..." -ForegroundColor Green
Start-Process "src-tauri\target\debug\neo-resonance.exe"
