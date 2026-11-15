# Sourced by the scripts that drive the desktop's pointer or draw in a window
# (play-test.sh, golden-images.sh, look-compare.sh): `refuse_without_desktop_input`
# stops the script while this desktop does not have the keyboard and mouse.
# Synergy shares them with another computer; while its active screen is the
# other one, injected input goes there, and Synergy keeps this desktop's
# pointer at the primary screen's centre, pulling it back after every warp.
# The harness then fails its pointer steps for no fault of ours.
#
# Two checks, both read-only unless the pointer sits at the centre:
# - Synergy's log: its last "leaving screen" comes after its last "entering
#   screen" (or a server start, which begins on this screen).
# - The pointer sits at the primary screen's centre and does not stay where
#   SetCursorPos puts it (scripts/pointer-probe.ps1; the pointer goes back).
#
# S2_SYNERGY_LOG names another log, S2_POINTER_PROBE another probe command
# (the tests use both; it prints "free X Y" or "pinned X Y").

synergy_log_default=/mnt/c/ProgramData/Synergy/logs/synergy.log

# The last line of the log that says which screen has the input: "leaving
# screen", "entering screen" or "started server". Empty without a log.
synergy_last_screen_line() {
  local log="${S2_SYNERGY_LOG:-$synergy_log_default}"
  [[ -r "$log" ]] || return 0
  tr -d '\r' <"$log" | grep -E -- '- (leaving screen|entering screen|started server)' | tail -n 1
}

# One line from the pointer probe, or nothing where there is no Windows.
pointer_probe() {
  if [[ -n "${S2_POINTER_PROBE:-}" ]]; then
    $S2_POINTER_PROBE
    return
  fi
  command -v powershell.exe >/dev/null 2>&1 || return 0
  local here
  here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  powershell.exe -NoProfile -ExecutionPolicy Bypass \
    -File "$(wslpath -w "$here/pointer-probe.ps1")" 2>/dev/null | tr -d '\r'
}

refuse_without_desktop_input() {
  local line
  line="$(synergy_last_screen_line)"
  if [[ "$line" == *"leaving screen"* ]]; then
    {
      echo "$(basename "$0"): Synergy's active screen is the other computer, not this desktop:"
      echo "  ${line}"
      echo "Injected input goes there and the pointer is held at the screen centre."
      echo "Move the pointer onto this desktop (Synergy writes \"entering screen\"), then start again."
    } >&2
    # EX_TEMPFAIL: try again later.
    exit 75
  fi
  local probe
  probe="$(pointer_probe)"
  if [[ "$probe" == pinned* ]]; then
    {
      echo "$(basename "$0"): the pointer sits at the screen centre and does not follow SetCursorPos (${probe#pinned }):"
      echo "  another program (Synergy with the other computer active) holds it there."
      echo "Give this desktop the mouse, then start again."
    } >&2
    exit 75
  fi
}
