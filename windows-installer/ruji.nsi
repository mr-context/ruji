; Installeur Windows de Ruji (NSIS).
; Compile avec : makensis windows-installer/ruji.nsi
; Nécessite d'avoir déjà buildé target/x86_64-pc-windows-msvc/release/ruji.exe.

!include "MUI2.nsh"

Name "Ruji"
OutFile "..\target\x86_64-pc-windows-msvc\release\RujiSetup.exe"
InstallDir "$LOCALAPPDATA\Ruji"
InstallDirRegKey HKCU "Software\Ruji" "InstallDir"
RequestExecutionLevel user

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\ruji.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Lancer Ruji maintenant"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "French"

Section "Ruji" SecRuji
    SetOutPath "$INSTDIR"
    File "..\target\x86_64-pc-windows-msvc\release\ruji.exe"

    WriteRegStr HKCU "Software\Ruji" "InstallDir" "$INSTDIR"

    ; Lance Ruji automatiquement à chaque ouverture de session Windows, comme un
    ; service en tâche de fond — pas besoin de le relancer à la main.
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Ruji" "$INSTDIR\ruji.exe"

    CreateDirectory "$SMPROGRAMS\Ruji"
    CreateShortcut "$SMPROGRAMS\Ruji\Ruji.lnk" "$INSTDIR\ruji.exe"
    CreateShortcut "$SMPROGRAMS\Ruji\Désinstaller.lnk" "$INSTDIR\uninstall.exe"

    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Entrée dans "Applications installées" (Panneau de configuration)
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ruji" "DisplayName" "Ruji"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ruji" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ruji" "InstallLocation" "$INSTDIR"
SectionEnd

Section "Uninstall"
    ; Ferme Ruji s'il tourne, sinon le .exe ne peut pas être supprimé.
    ExecWait 'taskkill /IM ruji.exe /F'

    Delete "$INSTDIR\ruji.exe"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\Ruji\Ruji.lnk"
    Delete "$SMPROGRAMS\Ruji\Désinstaller.lnk"
    RMDir "$SMPROGRAMS\Ruji"

    DeleteRegKey HKCU "Software\Ruji"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Ruji"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ruji"

    ; Fichiers du modèle extraits au premier lancement (voir preparer_dossier_modeles
    ; dans main.rs) — nettoyés à la désinstallation eux aussi.
    RMDir /r "$LOCALAPPDATA\ruji"
SectionEnd
