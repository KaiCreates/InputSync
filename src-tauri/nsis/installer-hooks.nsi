; InputSync NSIS installer hooks

; ── Pre-install: inform user if WebView2 will be installed ──────────────────
; With offlineInstaller mode, WebView2 is bundled — no download needed.
; This hook just sets user expectations if WebView2 isn't already present.
!macro NSIS_HOOK_PREINSTALL
  ; Check per-machine WebView2 (written by EdgeUpdate, a 32-bit service)
  ReadRegStr $0 HKLM \
    "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" \
    "pv"
  StrCmp $0 "" 0 WebView2Found

  ; Check per-user WebView2
  ReadRegStr $0 HKCU \
    "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" \
    "pv"
  StrCmp $0 "" 0 WebView2Found

  ; WebView2 not found — bundled installer will handle it (no download needed)
  MessageBox MB_ICONINFORMATION|MB_OK \
    "InputSync requires the Microsoft Edge WebView2 Runtime.$\r$\n$\r$\n\
The runtime is bundled with this installer and will be set up automatically.$\r$\n\
No internet connection is required.$\r$\n$\r$\n\
Click OK to continue." \
    /SD IDOK
  Goto WebView2Done

  WebView2Found:
    ; Already installed — nothing extra to do
  WebView2Done:
!macroend
