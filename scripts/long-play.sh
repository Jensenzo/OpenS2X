#!/usr/bin/env bash
# The long play (docs/test-coverage.md, "The long play"): the Windows viewer
# plays a game from tick 0 for 30 minutes, every frame timed, and the run
# fails when what grows with a long game slows it down. A step of the release
# net (docs/maintaining.md) since 0.11.4: the bone-palette buffer that grew
# for good twice slowed frames from 16.6 ms to 45..59 ms after half an hour
# (until 2026-11-04, and from 0.9.3 to 0.11.3), while every short benchmark
# stayed green.
#
#   scripts/long-play.sh                   build, run 30 minutes, check
#   NO_BUILD=1 scripts/long-play.sh        run the viewer built last
#   scripts/long-play.sh --check DIR       check a run's report.txt and log.txt
#
# The scene (as on 2026-11-04 and since): MP_4P_Close_Frontier, four
# players, a Hard computer player on the player's slot, game speed 3, the 3D
# view at 2560 x 1440 held to the display's refresh, muted, every soak
# invariant after every tick (`s2-viewer --free-game ... --bench-seconds`).
# LONG_PLAY_SECONDS shortens it (a try, not the release's run).
# LONG_PLAY_MAP plays another free game map, LONG_PLAY_ARGS adds viewer
# arguments (Windows paths; e.g. a project's --assets and --tree-model) and
# LONG_PLAY_OUT writes elsewhere: a measurement, not the release's run.
#
# The limits, for the whole run:
#   invariants broken            0
#   the lowest minute's pace     at least 29.5 ticks/s (speed 3: 30)
#   frame p99                    at most 25 ms (2026-11-04: 20.5; refresh 16.7)
#   frames over 100 ms           none, but one per change of the display's
#                                power state during the run (Windows'
#                                Kernel-Power event 566: the power plan turns
#                                the display off, or a session wakes it), each
#                                at most 500 ms: such a change stalls one
#                                present by about 300 ms (2026-11-17: 325 ms at
#                                the second of the event, 15 minutes into the
#                                run, before and after the palette fix)
#   the bone-palette buffer      at most 12.5 MB at the end (the slowdown's
#                                cause)
#   palette slots of no owner    none (the "P" lines: rest, poses, buildings)
#
# Needs S2_GAME_DIR and a desktop nobody uses: a timed run (leave the PC
# alone; another program's load shows in the times). The display is held on
# for the run and woken first by a pointer move of 0 (scripts/display-hold.ps1:
# with it off every frame waits about 264 ms). The viewer keeps its
# settings, profile and saves in target/long-play/profile (only the seed
# profile at the start, `s2-app profile-seed`),
# never the player's; the run fails when the player's files changed their
# modified time during it (scripts/player-files-guard.sh).
#
# The PC's load is recorded beside the frames (scripts/pc-load.ps1: a line a
# second in pc-load.txt with the whole machine's CPU, disks, the graphics
# card's 3D engine and the busiest processes not ours), and the check prints
# it for every frame over 100 ms and for the whole run: a slow frame stays a
# failure, but the report says whether the PC was plainly busy with
# something else at it. LOAD_HEAVY_CORE (100, % of one core) and
# LOAD_BUSY_CPU (25, % of the machine) set what it calls heavy and busy.
set -euo pipefail
cd "$(dirname "$0")/.."
# A process not ours using this many % of one core in a second is "heavy",
# and a second with the whole machine at this % is "busy" (pc_load).
LOAD_HEAVY_CORE=${LOAD_HEAVY_CORE:-100}
LOAD_BUSY_CPU=${LOAD_BUSY_CPU:-25}

# The PC's load beside the frames (scripts/pc-load.ps1, a line a second in
# pc-load.txt; the viewer's log says the wall clock of its 0 s, "T"): for
# each frame over 100 ms the samples around it, and over the whole run the
# seconds a process not ours was heavy. Information beside the limits, not a
# limit: a slow frame stays a failure, and the report says whether the PC
# was plainly busy with something else at it (2026-11-23: one frame of
# 157 ms and no record of what else ran).
pc_load() {
  local dir=$1 load=$1/pc-load.txt t0
  if [[ ! -s "$load" ]]; then
    echo "long-play: PC load: not recorded"
    return 0
  fi
  t0=$(awk '$1 == "T" { print $2; exit }' "$dir/log.txt")
  tr -d '\r' <"$load" | awk -v heavy="$LOAD_HEAVY_CORE" -v busy="$LOAD_BUSY_CPU" '
    $1 == "L" { n++; cpu = $4 + 0
      if (cpu >= busy) busy_s++
      for (i = 1; i <= NF; i++) if ($i == "top") break
      for (i = i + 1; i + 2 <= NF; i += 3) if ($(i + 2) + 0 >= heavy) { heavy_s[$i]++; seen = 1 }
      self += $11; cpu_sum += cpu
      if (cpu > cpu_max) cpu_max = cpu }
    END {
      if (!n) { print "long-play: PC load: no samples"; exit }
      printf "long-play: PC load: %d samples, mean CPU %.0f %%, highest %.0f %%, %d s at %d %% or more, sampler %.0f %% of a core\n", n, cpu_sum / n, cpu_max, busy_s, busy, self / n
      line = ""
      for (p in heavy_s) line = line sprintf(" %s %d s,", p, heavy_s[p])
      if (seen) print "long-play: PC load: processes not ours at a core or more:" line
      else print "long-play: PC load: no process not ours at a core or more" }'
  [[ -n "$t0" ]] || { echo "long-play: PC load: the log has no start time (T)"; return 0; }
  local at
  while read -r at; do
    tr -d '\r' <"$load" | awk -v w="$(awk -v t="$t0" -v a="$at" 'BEGIN { printf "%.0f", t + a * 1000 }')" -v at="$at" '
      $1 == "L" || $1 == "G" { if ($2 + 0 >= w && !after) { after = $0; next_t = $2 }
                               else if ($2 + 0 < w) { if ($1 == "L") before = $0; else gbefore = $0 }
                               if ($1 == "G" && $2 == next_t) gafter = $0 }
      END {
        print "long-play: PC at the frame at " at " s (samples end before and after it):"
        if (before) print "long-play:   " before; if (gbefore) print "long-play:   " gbefore
        if (after) print "long-play:   " after; if (gafter) print "long-play:   " gafter
        if (!before && !after) print "long-play:   no sample" }'
    if [[ -s "$dir/windows-events.txt" ]]; then
      awk -v w="$(awk -v t="$t0" -v a="$at" 'BEGIN { printf "%.0f", t + a * 1000 }')" '
        $1 + 0 <= w && w - $1 <= 3000 { $1 = sprintf("%.2f s before it:", (w - $1) / 1000); print "long-play:   Windows: " $0 }' "$dir/windows-events.txt"
    fi
  done < <(tr -d '\r' <"$dir/report.txt" | { grep '^stall F ' || true; } | awk '{ print $3 }')
}

# The limits on a run's report and log; prints each with its value.
check() {
  local dir=$1 report=$1/report.txt log=$1/log.txt fail=0
  [[ -f "$report" && -f "$log" ]] || { echo "long-play: no report or log in $dir" >&2; return 1; }
  value() { tr -d '\r' <"$report" | awk -v k="$1" '$1 == k && !seen { print $3; seen = 1 }'; }
  limit() { # name value op bound
    if awk -v v="$2" -v b="$4" -v op="$3" 'BEGIN { exit !((op == "<=" && v <= b) || (op == ">=" && v >= b) || (op == "==" && v == b)) }'; then
      echo "long-play: $1 $2 (limit $3 $4)"
    else
      echo "long-play: FAIL $1 $2 (limit $3 $4)"
      fail=1
    fi
  }
  local broken p99 over100 lowest palettes other seconds worst changes
  broken=$(value invariants-broken)
  worst=$(value frame-ms-worst)
  # The display's power changes the run saw (written by the run; none
  # known without the file).
  changes=0
  [[ -f "$dir/display-events.txt" ]] && changes=$(grep -c . "$dir/display-events.txt" || true)
  p99=$(value frame-ms-p99)
  over100=$(value frames-over-100ms)
  lowest=$(value ticks-per-second-lowest-minute)
  seconds=$(value seconds)
  palettes=$(tr -d '\r' <"$log" | { grep '^S ' || true; } | tail -1 | sed -n 's/.*palettes \([0-9.]*\) MB.*/\1/p')
  other=$(tr -d '\r' <"$log" | { grep '^P ' || true; } | sed -n 's/.*, other \(-\?[0-9]*\)$/\1/p' | sort -n | tail -1)
  echo "long-play: $seconds s played"
  limit "invariants broken" "${broken:-missing}" == 0
  limit "lowest minute (ticks/s)" "${lowest:-0}" ">=" 29.5
  limit "frame p99 (ms)" "${p99:-999}" "<=" 25
  echo "long-play: display power changes during the run: $changes"
  limit "frames over 100 ms" "${over100:-missing}" "<=" "$changes"
  if [[ "${over100:-0}" != 0 ]]; then
    limit "worst frame (ms)" "${worst:-999}" "<=" 500
  fi
  # What the view read after the game's start (models3d::preload), and for
  # each frame over 100 ms what it first read in it or just before: the
  # object of a first-use stall (0.12.2: 107 ms when the first well's site
  # appeared).
  echo "long-play: first uses during play: $(value first-uses-in-play)"
  local line uses grew
  while read -r line; do
    uses=$(grep -o '\[first use: [^]]*\]' <<<"$line" | sed 's/^\[first use: //; s/\]$//' | paste -sd, - || true)
    grew=$(grep -o '\[palette buffer grew [^]]*\]' <<<"$line" | tr -d '[]' || true)
    echo "long-play: frame over 100 ms at $(sed 's/^stall F //; s/ ms:.*/ ms/' <<<"$line"); first use near it: ${uses:-none}${grew:+; $grew}"
  done < <(tr -d '\r' <"$report" | { grep '^stall ' || true; })
  pc_load "$dir"
  limit "palette buffer at the end (MB)" "${palettes:-999}" "<=" 12.5
  limit "palette slots of no owner (most)" "${other:-0}" "<=" 0
  return $fail
}

if [[ "${1:-}" == "--check" ]]; then
  check "$2"
  exit
fi

source scripts/recording-guard.sh
refuse_while_recording
source scripts/player-files-guard.sh
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
out=${LONG_PLAY_OUT:-target/long-play}
seconds=${LONG_PLAY_SECONDS:-1800}
map=${LONG_PLAY_MAP:-MP_4P_Close_Frontier}
extra=()
if [[ -n "${LONG_PLAY_ARGS:-}" ]]; then read -r -a extra <<<"$LONG_PLAY_ARGS"; fi
if [[ -z "${NO_BUILD:-}" ]]; then
  cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
  cargo build --release -p s2-app
fi
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
# The game data: the Windows side's own copy of the install when it holds
# the same files (as scripts/play-test.sh: a first read over \\wsl.localhost
# stalls its frame by 30 to 150 ms).
game="$S2_GAME_DIR"
profile_w="$(powershell.exe -NoProfile -Command '[Environment]::GetFolderPath("UserProfile")' | tr -d '\r')"
local_copy="$(wslpath -u "$profile_w")/Games/S2-data"
listing() { (cd "$1" && find . -type f -printf '%P %s\n' | LC_ALL=C sort); }
if [[ -d "$local_copy/data" ]] && cmp -s <(listing "$S2_GAME_DIR") <(listing "$local_copy"); then
  game="$local_copy"
fi
echo "long-play: game data from $game, $map, $seconds s${extra[*]:+, ${extra[*]}}"
rm -rf "$out"
mkdir -p "$out/profile/users" "$out/profile/saves"
# The seed profile (the first start's values at the Very high preset), as
# the golden images' scenes have.
target/release/s2-app profile-seed "$out/profile/users/default01.cfg"
player_files_note
# The display held on for the run and woken first (scripts/display-hold.ps1):
# with it off after the power plan's idle timeout every present waits about
# 264 ms (2026-11-20: the release's run after a morning nobody used the PC,
# 6,807 frames of 263 ms). The wake is a pointer move of 0, input; its line
# goes to the output, which a desktop guard reads (the harness's own input).
hold_stop="$out/display-hold.stop"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w "$PWD/scripts/display-hold.ps1")" \
  -StopFile "$(wslpath -w "$PWD/$hold_stop")" -Seconds $((seconds + 900)) \
  </dev/null >"$out/display-hold.log" 2>&1 &
hold_pid=$!
release_display() {
  touch "$hold_stop" "$out/pc-load.stop"
  wait "$hold_pid" 2>/dev/null || true
}
trap release_display EXIT
for _ in $(seq 60); do
  grep -q "display-hold: \(held\|the request\)" "$out/display-hold.log" 2>/dev/null && break
  sleep 1
done
tr -d '\r' <"$out/display-hold.log"
grep -q "display-hold: held" "$out/display-hold.log" || { echo "long-play: the display is not held" >&2; exit 1; }
# The display on for good before the window opens (the keeper's STAY_MS).
sleep 5
started=$(date +%s)
# The PC's load a second beside the frames (scripts/pc-load.ps1): what else
# ran is in the report next to any slow frame.
load_stop="$out/pc-load.stop"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w "$PWD/scripts/pc-load.ps1")" \
  -Out "$(wslpath -w "$PWD/$out/pc-load.txt")" -StopFile "$(wslpath -w "$PWD/$load_stop")" \
  -Seconds $((seconds + 900)) -Ours "$(basename "$copy" .exe)" \
  </dev/null >"$out/pc-load.log" 2>&1 &
load_pid=$!
"$copy" --game-dir "$(wslpath -w "$game")" \
  --profile-dir "$(wslpath -w "$PWD/$out/profile")" \
  --save-dir "$(wslpath -w "$PWD/$out/profile/saves")" \
  --free-game "$map" --ai 0=3 --3d --speed 3 --size 2560x1440 "${extra[@]}" \
  --bench-vsync --bench-seconds "$seconds" --bench-invariants --mute \
  --bench-report "$(wslpath -w "$PWD/$out/report.txt")" \
  --bench-log "$(wslpath -w "$PWD/$out/log.txt")" \
  </dev/null >"$out/viewer.log" 2>&1 || true
touch "$load_stop"
wait "$load_pid" 2>/dev/null || true
# Windows' display session changes in the run's time (see the limits).
powershell.exe -NoProfile -Command "Get-WinEvent -FilterHashtable @{LogName='System'; Id=566; StartTime=[DateTimeOffset]::FromUnixTimeSeconds($started).LocalDateTime; EndTime=[DateTimeOffset]::FromUnixTimeSeconds($(date +%s)).LocalDateTime} -ErrorAction SilentlyContinue | ForEach-Object { '{0:o} {1}' -f \$_.TimeCreated.ToUniversalTime(), (\$_.Message -replace '\s+', ' ') }" \
  2>/dev/null | tr -d '\r' | grep . >"$out/display-events.txt" || true
# Windows' licence service started in the run's time (ClipSVC, trigger-
# started by the Store's licence poll every 15 minutes): 0.14.0's run and a
# quiet run of 2026-11-23 each had a cluster of render-prepare frames of
# 150..280 ms starting 0.26 s after one (two of the eight such starts in
# four runs). The check names one that came just before a frame over 100 ms.
powershell.exe -NoProfile -Command "Get-WinEvent -FilterHashtable @{LogName='Microsoft-Client-Licensing-Platform/Admin'; Id=100; StartTime=[DateTimeOffset]::FromUnixTimeSeconds($started).LocalDateTime; EndTime=[DateTimeOffset]::FromUnixTimeSeconds($(date +%s)).LocalDateTime} -ErrorAction SilentlyContinue | ForEach-Object { '{0} {1}' -f ([DateTimeOffset]\$_.TimeCreated).ToUnixTimeMilliseconds(), (\$_.Message -replace '\s+', ' ') }" \
  2>/dev/null | tr -d '\r' | grep . >"$out/windows-events.txt" || true
release_display
player_files_check || exit 1
check "$out"
