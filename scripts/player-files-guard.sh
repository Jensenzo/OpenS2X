# Sourced by the scripts that start the viewer on this PC's desktop
# (golden-images.sh, play-test.sh, frame-bench.sh, look-compare.sh): nothing
# they start may write the player's own files, not even the same bytes.
# `player_files_note` reads the modified times of the player's settings file
# (%APPDATA%\s2-reborn\settings.txt), of every file in the profiles' folder
# (Documents\S2\users) and in the save folder (Documents\S2\saves) when the
# run starts; `player_files_check [WHAT]` reads them again (at the run's end,
# or after each of its programs, WHAT naming it) and fails when one differs,
# or a file came or went; the next check compares with the times it read.
# Only the names and times are read, never a file's contents, and the save
# folder's names are not printed.
#
# Where there is no Windows (Linux CI) both do nothing. S2_PLAYER_SETTINGS,
# S2_PLAYER_USERS and S2_PLAYER_SAVES name other folders (the test does).

player_files_where() {
  if [[ -n "${S2_PLAYER_USERS:-}${S2_PLAYER_SAVES:-}${S2_PLAYER_SETTINGS:-}" ]]; then
    printf '%s\n%s\n%s\n' "${S2_PLAYER_SETTINGS:-}" "${S2_PLAYER_USERS:-}" "${S2_PLAYER_SAVES:-}"
    return
  fi
  command -v powershell.exe >/dev/null 2>&1 || return 0
  local appdata="" docs=""
  { read -r appdata; read -r docs; } < <(powershell.exe -NoProfile -Command \
    '[Environment]::GetFolderPath("ApplicationData"); [Environment]::GetFolderPath("MyDocuments")' \
    | tr -d '\r') || true
  [[ -n "$appdata" && -n "$docs" ]] || return 0
  printf '%s\n%s\n%s\n' "$(wslpath -u "$appdata")/s2-reborn/settings.txt" \
    "$(wslpath -u "$docs")/S2/users" "$(wslpath -u "$docs")/S2/saves"
}

# One line per file: "<kind> <modified time, ns> <name>" (kind settings,
# users or saves).
player_files_times() {
  local settings="" users="" saves=""
  { read -r settings; read -r users; read -r saves; } <<<"${player_files_paths:-}" || true
  if [[ -n "$settings" && -e "$settings" ]]; then
    echo "settings $(stat -c '%.9Y' "$settings") settings.txt"
  fi
  local kind dir
  for kind in users saves; do
    dir="${!kind}"
    [[ -n "$dir" && -d "$dir" ]] || continue
    find "$dir" -maxdepth 1 -type f -printf "$kind %T@ %f\n" | LC_ALL=C sort -k3
  done
}

player_files_note() {
  player_files_paths="$(player_files_where)"
  player_files_before="$(player_files_times)"
}

# Fails (exit status 1) when a time changed since the last note or check.
player_files_check() {
  local after
  after="$(player_files_times)"
  [[ "$after" == "${player_files_before:-}" ]] && return 0
  {
    echo "$(basename "$0"): ${1:-the run} wrote the player's own files (their modified times changed):"
    diff <(echo "${player_files_before:-}") <(echo "$after") | grep '^[<>]' \
      | awk '{ if ($2 == "saves") print "  a file in the save folder"; else print "  " $2 ": " $4 }' \
      | LC_ALL=C sort -u
  } >&2
  player_files_before="$after"
  return 1
}
