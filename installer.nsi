Unicode true
Name "NeoResonance"
OutFile "C:\Users\shaur\NeoResonance\NeoResonance-Setup-v0.1.6.exe"
InstallDir "$LOCALAPPDATA\Programs\NeoResonance"
RequestExecutionLevel user

Function .onInit
    ; Force kill any running instances and child webviews before installation
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im NeoResonance.exe'
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im neo-resonance.exe'
    Sleep 1000
FunctionEnd

Page directory
Page instfiles

Section "Install"
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im NeoResonance.exe'
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im neo-resonance.exe'
    Sleep 1000

    SetOutPath "$INSTDIR"
    Delete "$INSTDIR\NeoResonance.exe"
    Delete "$INSTDIR\WebView2Loader.dll"

    File /oname=NeoResonance.exe "C:\Users\shaur\NeoResonance\src-tauri\target\release\neo-resonance.exe"
    File "C:\Users\shaur\NeoResonance\src-tauri\target\release\WebView2Loader.dll"
    
    CreateShortCut "$DESKTOP\NeoResonance.lnk" "$INSTDIR\NeoResonance.exe"
    CreateDirectory "$SMPROGRAMS\NeoResonance"
    CreateShortCut "$SMPROGRAMS\NeoResonance\NeoResonance.lnk" "$INSTDIR\NeoResonance.exe"
    CreateShortCut "$SMPROGRAMS\NeoResonance\Uninstall.lnk" "$INSTDIR\uninstall.exe"
    
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im NeoResonance.exe'
    ExecWait '"$SYSDIR\cmd.exe" /c taskkill /f /t /im neo-resonance.exe'
    Sleep 1000

    Delete "$DESKTOP\NeoResonance.lnk"
    Delete "$SMPROGRAMS\NeoResonance\NeoResonance.lnk"
    Delete "$SMPROGRAMS\NeoResonance\Uninstall.lnk"
    RMDir "$SMPROGRAMS\NeoResonance"
    
    Delete "$INSTDIR\NeoResonance.exe"
    Delete "$INSTDIR\WebView2Loader.dll"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
SectionEnd
