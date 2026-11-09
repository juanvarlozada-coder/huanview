; Huanview — Windows installer (NSIS), multi-idioma
!define APP "Huanview"
!define APPLOWER "huanview"
!define VERSION "0.1.5"
Name "${APP} ${VERSION}"
OutFile "..\..\dist\Huanview-0.1.5-installer-win64.exe"
Unicode true
ManifestDPIAware true
InstallDir "$LOCALAPPDATA\${APP}"
RequestExecutionLevel user

!include "MUI2.nsh"
!define MUI_ABORTWARNING
!define MUI_RESERVEFILE_LANGDLL

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; el diálogo de selección de idioma aparece al declarar varios idiomas
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "PortugueseBR"

; textos por idioma (accesos directos y resultados)
LangString LngInstalled  ${LANG_SPANISH}      "Huanview se ha instalado correctamente."
LangString LngInstalled  ${LANG_ENGLISH}      "Huanview has been installed successfully."
LangString LngInstalled  ${LANG_PORTUGUESEBR} "Huanview foi instalado com sucesso."
LangString LngUninstalled ${LANG_SPANISH}      "Huanview se ha desinstalado."
LangString LngUninstalled ${LANG_ENGLISH}      "Huanview has been uninstalled."
LangString LngUninstalled ${LANG_PORTUGUESEBR} "Huanview foi desinstalado."
LangString LngShortcutName ${LANG_SPANISH}      "${APP} (visor de imágenes)"
LangString LngShortcutName ${LANG_ENGLISH}      "${APP} (image viewer)"
LangString LngShortcutName ${LANG_PORTUGUESEBR} "${APP} (visualizador de imagens)"

Icon "icon.ico"
UninstallIcon "icon.ico"
InstallDirRegKey HKCU "Software\${APP}" "InstallDir"

Section "Install"
  SetOutPath "$INSTDIR"
  IfFileExists "$INSTDIR" 0 +2
  CreateDirectory "$INSTDIR"
  File "..\..\target\x86_64-pc-windows-gnu\release\${APPLOWER}.exe"
  File "icon.ico"

  ; accesos directos con nombre traducido al idioma elegido
  CreateDirectory "$SMPROGRAMS"
  CreateShortcut "$SMPROGRAMS\$(LngShortcutName).lnk" "$INSTDIR\${APPLOWER}.exe" "" "$INSTDIR\icon.ico"
  CreateShortcut "$DESKTOP\$(LngShortcutName).lnk" "$INSTDIR\${APPLOWER}.exe" "" "$INSTDIR\icon.ico"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayName" "${APP}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}" "NoRepair" 1
SectionEnd

Function .onInstSuccess
  DetailPrint "$(LngInstalled)"
FunctionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APPLOWER}.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\$(LngShortcutName).lnk"
  Delete "$SMPROGRAMS\${APP}.lnk"
  Delete "$DESKTOP\$(LngShortcutName).lnk"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"
  DeleteRegKey HKCU "Software\${APP}"
SectionEnd

Function un.onUninstSuccess
  DetailPrint "$(LngUninstalled)"
FunctionEnd
