# Sourced by the scripts that drive a window or the installed game on this PC
# (play-test.sh, golden-images.sh, view-map.sh, golden.sh, install-local.sh):
# `refuse_while_recording` stops the script while the original is being
# recorded with nobody at the PC (TOOL-trace TRACE-7.3). The recording's
# launcher keeps the flag file below for as long as it runs; a window that
# moves the desktop pointer or takes the foreground stops or spoils the
# recording. The flag is in the Windows user's folder of the WSL user's name;
# S2_RECORDING_FLAG names another flag file (the tests use one).

refuse_while_recording() {
  local flag="${S2_RECORDING_FLAG:-/mnt/c/Users/${USER:-$(id -un)}/s2trace/RECORDING-IN-PROGRESS}"
  [[ -e "$flag" ]] || return 0
  {
    echo "$(basename "$0"): a recording of the original runs on this PC (TRACE-7.3): $flag"
    tr -d '\r' <"$flag" | sed 's/^/  /'
    if [[ -n "$(find "$flag" -mmin +1440 2>/dev/null)" ]]; then
      echo "The flag is older than a day: if no launcher (auto_record.sh) runs, it is stale and may be removed by hand."
    else
      echo "Start again when the launcher has removed it."
    fi
  } >&2
  # EX_TEMPFAIL: try again later.
  exit 75
}
