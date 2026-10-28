; Huanview — Windows installer (NSIS)
!define APP "Huanview"
!define APPLOWER "huanview"
!define VERSION "0.1.0"
Name "${APP} ${VERSION}"
OutFile "..\..\dist\Huanview-0.1.0-installer-win64.exe"
Unicode true
ManifestDPIAware true
InstallDir "$LOCALAPPDATA\${APP}"
RequestExecutionLevel user
Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Icon "icon.ico"
UninstallIcon "icon.ico"
InstallDirRegKey HKCU "Software\${APP}" "InstallDir"

Section "Install"
  SetOutPath "$INSTDIR"
  IfFileExists "$INSTDIR" 0 +2
  CreateDirectory "$INSTDIR"
  File "..\..\target\x86_64-pc-windows-gnu\release\${APPLOWER}.exe"
  File "icon.ico"

  ; shortcuts
  CreateDirectory "$SMPROGRAMS"
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\${APPLOWER}.exe" "" "$INSTDIR\icon.ico"
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\${APPLOWER}.exe" "" "$INSTDIR\icon.ico"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayName" "${APP}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "NoRepair" 1

  ; associate common image types (optional, commented for minimalism)
  ; WriteRegStr HKCU "Software\Classes\Applications\huanview.exe\shell\open\command" "" '"$INSTDIR\huanview.exe" "%1"'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APPLOWER}.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP}.lnk"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"
  DeleteRegKey HKCU "Software\${APP}"
SectionEnd
