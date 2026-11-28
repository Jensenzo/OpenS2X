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
# LONG_PLAY_EXE runs another build of the viewer (NO_BUILD=1 with it).
# PROBE_WINDOW=1 opens a second, tiny window of ours beside the game (the
# same exe, `--probe-window`: one triangle, its own frame log probe.txt with
# the wall clock, on the left display) to tell a stall of the graphics stack
# for everyone from one of the game alone (2026-11-24).
#
# The limits, for the whole run:
#   invariants broken            0
#   the lowest minute's pace     at least 29.5 ticks/s (speed 3: 30)
#   frame p99                    at most 25 ms (2026-11-04: 20.5; refresh 16.7)
#   frames over 100 ms           none (the PC's licence check aside, below),
#                                but one per change of the display's
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
#   Windows' licence events      read (windows-events.txt says "read N")
#
# The PC's licence check (2026-11-26): a frame over 100 ms that begins 0.2
# to 1.0 s after a start of Windows' licence service (ClipSVC, the Store's
# poll every 15 minutes) is the PC's, not the game's: it is listed with its
# slowest step and its time after the start, and does not fail the run. A
# frame over 100 ms that begins at most 1.0 s after the end of such a frame
# belongs to it. Every 30-minute run on 2026-11-23..26 that failed, failed on
# such frames only (the release run of 26 Nov, the measurements of 24..26 Nov):
# with our graphics calls cut by the CPU instance path the wait moved into a
# game tick, the submit and once into a separate window, so it is Windows'.
#
#   scripts/long-play.sh --self-test      the check on made-up runs
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
    # The render steps around it (the viewer's R lines, 2026-11-24): each
    # render frame's slowest step, its CPU and faults.
    tr -d '\r' <"$dir/log.txt" | awk -v at="$at" '
      $1 == "R" && $2 + 0 >= at - 0.2 && $2 + 0 <= at + 0.3 {
        n = split($0, part, /\), /); best = 0; what = ""
        for (i = 1; i <= n; i++) {
          if (!match(part[i], /[a-z][a-z ,]* [0-9.]+ \(thread/)) continue
          s = substr(part[i], RSTART, RLENGTH - 8); v = s; sub(/.* /, "", v)
          if (v + 0 > best) { best = v + 0; what = part[i] }
        }
        sub(/^.*: /, "", what); sub(/^[^:]*MB: /, "", what)
        ws = $0; sub(/^.*working set /, "", ws); sub(/ MB:.*/, "", ws)
        printf "long-play:   render frame %s (%s s): slowest %s), working set %s MB\n", $6 + 0, $2, what, ws }'
    if [[ -s "$dir/probe.txt" ]]; then
      tr -d '\r' <"$dir/probe.txt" | awk -v w="$(awk -v t="$t0" -v a="$at" 'BEGIN { printf "%.0f", t + a * 1000 }')" '
        $1 == "F" && $2 + 0 >= w - 1000 && $2 + 0 <= w + 1500 { line = line sprintf(" %+.2f s %s ms,", ($2 - w) / 1000, $3) }
        $1 == "S" && $2 + 0 >= w - 1000 && $2 + 0 <= w + 2000 { if ($6 + 0 > worst) worst = $6 + 0; seen = 1 }
        END { if (!seen) print "long-play:   probe window: no record"
              else printf "long-play:   probe window: worst %.1f ms in the 3 s around it;%s\n", worst, (line ? line : " no frame over 25 ms") }'
    fi
    if [[ -s "$dir/windows-events.txt" ]]; then
      awk -v w="$(awk -v t="$t0" -v a="$at" 'BEGIN { printf "%.0f", t + a * 1000 }')" '
        $1 + 0 <= w && w - $1 <= 3000 { $1 = sprintf("%.2f s before it:", (w - $1) / 1000); print "long-play:   Windows: " $0 }' "$dir/windows-events.txt"
    fi
  done < <(tr -d '\r' <"$dir/report.txt" | { grep '^stall F ' || true; } | awk '{ print $3 }')
}

# Whether the run's Windows licence events were read: "read N" first (since
# 2026-11-26, scripts/licence-events.ps1), or, in a run's files from before,
# at least one start listed.
licence_read() {
  local events=$1/windows-events.txt
  [[ -s "$events" ]] || return 1
  tr -d '\r' <"$events" | grep -q '^read [0-9]\+$\|^[0-9]\+ ClipSVC service is starting'
}

# Each frame over 100 ms of the report, judged: "licence AT MS AFTER FRAME
# HOW FRAME SLOWEST" for the PC's licence check (see the top: AFTER is the
# seconds from the service's start to the frame's beginning; HOW "start" for
# one 0.2 to 1.0 s after it, "follows" for one that belongs to the one
# before), "other AT MS - FRAME SLOWEST" for the rest. A
# frame's beginning is the run's 0 s ("T") + its time (logged at its end) -
# its length. SLOWEST is the frame's slowest part, and the render frame's
# slowest step ("R" lines) when the log has one.
stall_kinds() {
  local dir=$1 side
  mkdir -p target/tmp
  side=$(mktemp -p target/tmp stall-kinds.XXXXXX)
  tr -d '\r' <"$dir/log.txt" | awk '
    $1 == "T" && t0 == "" { t0 = $2 }
    $1 == "R" {
      f = $6; sub(/,$/, "", f); n = split($0, part, /\), /); best = 0; what = ""
      for (i = 1; i <= n; i++) {
        if (!match(part[i], /[a-z][a-z ,]* [0-9.]+ \(thread/)) continue
        s = substr(part[i], RSTART, RLENGTH - 8); v = s; sub(/.* /, "", v)
        if (v + 0 > best) { best = v + 0; what = s }
      }
      if (what != "") { sub(/^.*: /, "", what); sub(/^[^:]*MB: /, "", what); render[f] = what }
    }
    END { print "T", t0; for (f in render) print "R", f, render[f] }' >"$side"
  { tr -d '\r' <"$dir/windows-events.txt" 2>/dev/null || true; } | awk '$2 == "ClipSVC" { print "C", $1 }' >>"$side"
  tr -d '\r' <"$dir/report.txt" | { grep '^stall F ' || true; } | awk -v side="$side" '
    BEGIN {
      while ((getline l < side) > 0) {
        split(l, w, " ")
        if (w[1] == "T") t0 = w[2]
        else if (w[1] == "C") starts[++nc] = w[2] + 0
        else if (w[1] == "R") { r = l; sub(/^R [0-9]+ /, "", r); render[w[2]] = r }
      }
    }
    {
      at = $3 + 0; ms = $10 + 0; frame = $6; sub(/,$/, "", frame)
      end_ms = t0 + at * 1000; begin = end_ms - ms
      line = $0; sub(/^[^)]*\): [0-9.]+ ms:/, "", line); gsub(/\[[^]]*\],?/, "", line)
      best = -1; slowest = ""
      while (match(line, /[a-z][a-z ,]*[a-z] [0-9]+(\.[0-9]+)?/)) {
        s = substr(line, RSTART, RLENGTH); v = s; sub(/.* /, "", v); name = s; sub(/ [0-9.]+$/, "", name)
        if (v + 0 > best) { best = v + 0; slowest = name " " v " ms" }
        line = substr(line, RSTART + RLENGTH)
      }
      if (frame in render) slowest = slowest " (render: " render[frame] " ms)"
      if (slowest == "") slowest = "not known"
      after = -1
      for (i = 1; i <= nc; i++) if (starts[i] <= begin && (after < 0 || begin - starts[i] < after)) after = begin - starts[i]
      kind = "other"; how = "start"
      if (t0 != "" && after >= 200 && after <= 1000) kind = "licence"
      else if (t0 != "" && last_end != "" && begin >= last_end - 1 && begin - last_end <= 1000) { kind = "licence"; how = "follows" }
      if (kind == "licence") { last_end = end_ms; printf "licence %s %s %.2f %s %s %s\n", $3, $10, after / 1000, how, frame, slowest }
      else printf "other %s %s - %s %s\n", $3, $10, frame, slowest
    }'
  rm -f "$side"
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
  # The PC's licence check (see the top): listed, not counted. A run whose
  # Windows events were not read fails, since then no frame can be told.
  local kinds licence others other_worst
  if licence_read "$dir"; then
    echo "long-play: Windows' licence-service starts read: $(tr -d '\r' <"$dir/windows-events.txt" | grep -c ' ClipSVC service is starting' || true)"
  else
    echo "long-play: FAIL Windows' licence events not read ($dir/windows-events.txt)"
    fail=1
  fi
  kinds=$(stall_kinds "$dir")
  if ! licence_read "$dir"; then kinds=$(sed 's/^licence /other /' <<<"$kinds"); fi
  licence=$(grep -c '^licence ' <<<"$kinds" || true)
  others=$(grep -c '^other ' <<<"$kinds" || true)
  echo "long-play: frames over 100 ms ${over100:-missing}, of them the PC's licence check $licence"
  local how
  while read -r _ at ms after how frame slowest; do
    [[ $how == follows ]] && how=" (within 1 s of the one before)" || how=
    echo "long-play: the PC's licence check: frame $frame at $at s, $ms ms, began $after s after ClipSVC's start$how; slowest $slowest"
  done < <(grep '^licence ' <<<"$kinds" || true)
  # A report whose stall lines miss frames over 100 ms counts them as others.
  if [[ -n "${over100:-}" ]] && ((over100 - licence > others)); then others=$((over100 - licence)); fi
  [[ -n "${over100:-}" ]] || others=missing
  limit "frames over 100 ms not the licence check" "$others" "<=" "$changes"
  if [[ "$others" != 0 ]]; then
    other_worst=$({ grep '^other ' <<<"$kinds" || true; } | awk '$3 + 0 > w { w = $3 + 0 } END { print w + 0 }')
    ((licence == 0)) && other_worst=${worst:-999}
    limit "worst frame not the licence check (ms)" "${other_worst:-999}" "<=" 500
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

# The check on made-up runs (docs/test-coverage.md, "The long play"): each
# case a report, a log and Windows' events, and the verdict it must give.
self_test() {
  local root t0=1000000000000 failed=0
  mkdir -p target/tmp
  root=$(mktemp -d -p target/tmp long-play-self-test.XXXXXX)
  # make NAME EVENTS STALL... : STALL is "BEGIN_AFTER_T0_MS:MS"; EVENTS is
  # "none" (no file), "empty", or a list of "read", "old" and starts in ms.
  make() {
    local dir=$root/$1 events=$2 n=0 worst=0 st b ms at
    shift 2
    mkdir -p "$dir"
    {
      echo "T $t0 unix ms at 0 s"
      echo "S 1800.00 s: palettes 6.2 MB"
    } >"$dir/log.txt"
    : >"$dir/stalls"
    for st in "$@"; do
      b=${st%%:*} ms=${st##*:}
      at=$(awk -v b="$b" -v m="$ms" 'BEGIN { printf "%.2f", (b + m) / 1000 }')
      echo "stall F $at s, frame $((1000 + n)), tick 500 (+2): $ms ms: interface 2.0, settlers, buildings, wares, animation 5.0, render prepare $((${ms%.*} - 10)).0, [faults 10, working set 900 MB]," >>"$dir/stalls"
      echo "R $at s, render frame $((1000 + n)), unix ms 0, working set 900 -> 900 MB: wait and extract 0.5 (thread -, process 1.0 k 0, faults 0), bind groups $((${ms%.*} - 20)).0 (thread -, process 2.0 k 0, faults 0), submit 1.0 (thread -, process 1.0 k 0, faults 0)" >>"$dir/log.txt"
      n=$((n + 1))
      awk -v m="$ms" -v w="$worst" 'BEGIN { exit !(m > w) }' && worst=$ms
    done
    {
      printf 'seconds = 1800.00\ninvariants-broken = 0\nticks-per-second-lowest-minute = 30.0\n'
      printf 'frame-ms-p99 = 19.0\nframes-over-100ms = %s\nframe-ms-worst = %s\nfirst-uses-in-play = 0\n' "$n" "$worst"
      cat "$dir/stalls"
    } >"$dir/report.txt"
    rm "$dir/stalls"
    case $events in
      none) ;;
      empty) : >"$dir/windows-events.txt" ;;
      *)
        local e
        for e in $events; do
          case $e in
            read) echo "read $(wc -w <<<"$events")" ;;
            old) ;;
            *) echo "$((t0 + e)) ClipSVC service is starting. Caller: TriggerStarted" ;;
          esac
        done >"$dir/windows-events.txt"
        ;;
    esac
  }
  expect() { # NAME pass|fail PATTERN...
    local name=$1 want=$2 out got
    shift 2
    if out=$(check "$root/$name" 2>&1); then got=pass; else got=fail; fi
    local p ok=1
    for p in "$@"; do grep -q -- "$p" <<<"$out" || ok=0; done
    if [[ $got == "$want" && $ok == 1 ]]; then
      echo "long-play self-test: ok   $name ($got)"
    else
      echo "long-play self-test: FAIL $name: wanted $want, got $got"
      printf '%s\n' "$out" | sed 's/^/    /'
      failed=1
    fi
  }
  make quiet "read" ; expect quiet pass "starts read: 0"
  make licence "read 900000" 900500:150 ; expect licence pass "licence check 1" "began 0.50 s after ClipSVC's start; slowest render prepare 140.0 ms (render: bind groups 130.0 ms)" "not the licence check 0 "
  make window-edges "read 900000 1800000" 900200:120 1801000:130 ; expect window-edges pass "licence check 2"
  make too-soon "read 900000" 900150:150 ; expect too-soon fail "FAIL frames over 100 ms not the licence check 1"
  make too-late "read 900000" 901050:150 ; expect too-late fail "FAIL frames over 100 ms not the licence check 1"
  make cluster "read 900000" 900300:150 900650:400 901800:130 ; expect cluster pass "licence check 3" "began 1.80 s after ClipSVC's start (within 1 s of the one before)"
  make cluster-gap "read 900000" 900300:150 901500:130 ; expect cluster-gap fail "licence check 1" "FAIL frames over 100 ms not the licence check 1"
  make licence-and-other "read 900000" 900400:150 1200000:120 ; expect licence-and-other fail "licence check 1" "FAIL frames over 100 ms not the licence check 1"
  make long-licence "read 900000" 900400:600 ; expect long-licence pass "licence check 1"
  make old-format "old 900000" 900400:150 ; expect old-format pass "licence check 1"
  make no-events none ; expect no-events fail "FAIL Windows' licence events not read"
  make empty-events empty 900400:150 ; expect empty-events fail "FAIL Windows' licence events not read" "licence check 0"
  echo "2026-01-01T00:00:00 display" >"$root/no-events/display-events.txt"
  make display-change "read" 500000:300 ; echo "2026-01-01T00:00:00 display" >"$root/display-change/display-events.txt"
  expect display-change pass "not the licence check 1 (limit <= 1)" "worst frame not the licence check (ms) 300"
  make display-long "read 900000" 900400:150 500000:600 ; echo "2026-01-01T00:00:00 display" >"$root/display-long/display-events.txt"
  expect display-long fail "FAIL worst frame not the licence check (ms) 600"
  rm -rf "$root"
  return $failed
}

if [[ "${1:-}" == "--self-test" ]]; then
  self_test
  exit
fi

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
# LONG_PLAY_EXE runs another build of the viewer (the investigation's
# `--features frame-trace` build: a span log beside the log, a measurement).
windows_copy "${LONG_PLAY_EXE:-target/x86_64-pc-windows-gnu/release/s2-viewer.exe}"
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
probe_pid=
if [[ -n "${PROBE_WINDOW:-}" ]]; then
  "$copy" --probe-window "$(wslpath -w "$PWD/$out/probe.txt")" --bench-seconds $((seconds + 900)) \
    </dev/null >"$out/probe.log" 2>&1 &
  probe_pid=$!
fi
"$copy" --game-dir "$(wslpath -w "$game")" \
  --profile-dir "$(wslpath -w "$PWD/$out/profile")" \
  --save-dir "$(wslpath -w "$PWD/$out/profile/saves")" \
  --free-game "$map" --ai 0=3 --3d --speed 3 --size 2560x1440 "${extra[@]}" \
  --bench-vsync --bench-seconds "$seconds" --bench-invariants --mute \
  --bench-report "$(wslpath -w "$PWD/$out/report.txt")" \
  --bench-log "$(wslpath -w "$PWD/$out/log.txt")" \
  </dev/null >"$out/viewer.log" 2>&1 || true
touch "$load_stop"
[[ -n "$probe_pid" ]] && touch "$out/probe.txt.stop"
wait "$load_pid" 2>/dev/null || true
[[ -n "$probe_pid" ]] && { wait "$probe_pid" 2>/dev/null || true; }
# Windows' display session changes in the run's time (see the limits).
powershell.exe -NoProfile -Command "Get-WinEvent -FilterHashtable @{LogName='System'; Id=566; StartTime=[DateTimeOffset]::FromUnixTimeSeconds($started).LocalDateTime; EndTime=[DateTimeOffset]::FromUnixTimeSeconds($(date +%s)).LocalDateTime} -ErrorAction SilentlyContinue | ForEach-Object { '{0:o} {1}' -f \$_.TimeCreated.ToUniversalTime(), (\$_.Message -replace '\s+', ' ') }" \
  2>/dev/null | tr -d '\r' | grep . >"$out/display-events.txt" || true
# Windows' licence service started in the run's time (ClipSVC, trigger-
# started by the Store's licence poll every 15 minutes): 0.14.0's run and a
# quiet run of 2026-11-23 each had a cluster of render-prepare frames of
# 150..280 ms starting 0.26 s after one (two of the eight such starts in
# four runs). The check names one that came just before a frame over 100 ms.
# Since 2026-11-26 such a frame is the PC's licence check (see the top), and
# the file says "read N" first: without it the run fails.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w "$PWD/scripts/licence-events.ps1")" \
  -Start "$started" -End "$(date +%s)" \
  2>"$out/windows-events.log" | tr -d '\r' | grep . >"$out/windows-events.txt" || true
release_display
player_files_check || exit 1
check "$out"
