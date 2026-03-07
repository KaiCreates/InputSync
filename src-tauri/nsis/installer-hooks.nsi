; InputSync NSIS installer hooks
; Called by Tauri's generated NSIS installer at key lifecycle points.

; ── Check if WebView2 is already installed ──────────────────────────────────
!macro NSIS_HOOK_PREINSTALL
  ; Check per-machine WebView2 (GUID for Edge WebView2 Runtime)
  ReadRegStr $0 HKLM \
    "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" \
    "pv"
  StrCmp $0 "" 0 WebView2Found

  ; Check per-user WebView2
  ReadRegStr $0 HKCU \
    "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" \
    "pv"
  StrCmp $0 "" 0 WebView2Found

  ; WebView2 not found — warn user before download begins
  MessageBox MB_ICONINFORMATION|MB_OK \
    "InputSync requires the Microsoft Edge WebView2 Runtime.$\r$\n$\r$\n\
It was not found on this machine, so the installer will download and install it now (~100 MB).$\r$\n$\r$\n\
This may take a few minutes depending on your internet connection.$\r$\n\
The installer is NOT frozen — please wait.$\r$\n$\r$\n\
Click OK to continue." \
    /SD IDOK
  Goto WebView2Done

  WebView2Found:
    ; Already installed — nothing to do
  WebView2Done:
!macroend
