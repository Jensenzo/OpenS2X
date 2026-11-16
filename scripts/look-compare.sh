#!/usr/bin/env bash
# The automatic look comparison (TOOL-trace TRACE-8): the
# original's pictures beside ours, each pair at the same trace tick and
# with the same camera.
#
#   scripts/look-compare.sh DIR
#
# DIR is the investigation's shots folder of one run
# (s2-reference/screens/look-compare-auto/<date>): DIR/original/<stem>.png
# with DIR/original/<stem>.json, the camera file (tick, map, game, window,
# eye, dir, ...). For each, `s2-app shot` draws our game at that tick with
# that camera and the window's size into DIR/ours/<stem>.png, and the page
# DIR/index.html shows the pairs side by side. The game is the recorded
# start of its map with the computer player of `game` in slot 1 (weak:
# profile 1, normal: 2, hard: 3; AI=<slot>=<profile>,... overrides). A
# camera file with "fog_of_war": "off" (the original's game set up with fog
# of war off) gets ours with every field drawn as explored (`--explored`),
# one with "paused": true the paused view's layer (`--paused`).
# Nothing is written into the repository. Needs S2_GAME_DIR and a desktop
# (the viewer draws in a window, as for the golden images). Each picture's
# viewer keeps its settings and profile in an empty folder of its own
# (target/look-compare/profile/<stem>), never the player's; the run fails
# when the player's files changed their modified time during it
# (scripts/player-files-guard.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/recording-guard.sh
refuse_while_recording
source scripts/input-guard.sh
refuse_without_desktop_input
source scripts/player-files-guard.sh
: "${S2_GAME_DIR:?set S2_GAME_DIR to the game install}"
dir="${1:?usage: scripts/look-compare.sh DIR (the shots folder of one run)}"
dir="${dir%/}"
[[ -d "$dir/original" ]] || { echo "no $dir/original" >&2; exit 2; }
if [[ -z "${NO_BUILD:-}" ]]; then
  cargo build --release --target x86_64-pc-windows-gnu -p s2-viewer
  cargo build --release -p s2-app
fi
source scripts/windows-copy.sh
windows_copy target/x86_64-pc-windows-gnu/release/s2-viewer.exe
viewer="$copy"
app=target/release/s2-app
mkdir -p "$dir/ours" "$dir/log"
profiles=target/look-compare/profile
player_files_note

# One field of a camera file (a number, a string, or "w h" for window).
field() {
  python3 -I -c '
import json, sys
c = json.load(open(sys.argv[1]))
v = c.get(sys.argv[2])
if v is None:
    sys.exit(0)
print(" ".join(str(x) for x in v) if isinstance(v, list) else v)
' "$1" "$2"
}

rows=""
status=0
shopt -s nullglob
for json in "$dir"/original/*.json; do
  stem="$(basename "$json" .json)"
  original="$dir/original/$stem.png"
  tick="$(field "$json" tick)"
  map="$(field "$json" map)"
  game="$(field "$json" game)"
  read -r w h <<<"$(field "$json" window)"
  # The game's name may carry the run's set-up after it ("weakfogoff").
  case "${AI:-$game}" in
    weak*) ai="1=1" ;;
    normal*) ai="1=2" ;;
    hard*) ai="1=3" ;;
    *=*) ai="${AI}" ;;
    *) echo "$stem: unknown game '$game' (set AI=<slot>=<profile>)" >&2; status=1; continue ;;
  esac
  ours="$dir/ours/$stem.png"
  note=""
  # A picture the original took with fog of war off: ours with every field
  # drawn as explored (Q-290).
  extra=()
  fog="$(field "$json" fog_of_war)"
  [[ "$fog" == off ]] && extra+=(--explored)
  # A picture taken paused gets the paused view's layer (UI-GAME-1.11).
  [[ "$(field "$json" paused)" == True ]] && extra+=(--paused)
  rm -rf "${profiles:?}/$stem"
  if "$app" shot --map "$map" --ai "$ai" --tick "$tick" --camera "$json" \
      --size "${w}x${h}" --out "$ours" --viewer "$viewer" "${extra[@]}" \
      --profile-dir "$profiles/$stem" >"$dir/log/$stem.log" 2>&1; then
    echo "$stem: tick $tick, ours drawn"
  else
    echo "$stem: no picture of ours (log: $dir/log/$stem.log)" >&2
    note="no picture of ours: see log/$stem.log"
    status=1
  fi
  rows+="<section><h2>$stem</h2><p>$map, $game, trace tick $tick, ${w} x ${h}${fog:+, fog of war $fog}${note:+ &mdash; $note}</p>"
  rows+="<div class=pair><figure><a href=\"original/$stem.png\"><img src=\"original/$stem.png\" alt=\"original\"></a><figcaption>original</figcaption></figure>"
  rows+="<figure><a href=\"ours/$stem.png\"><img src=\"ours/$stem.png\" alt=\"ours\"></a><figcaption>s2-reborn</figcaption></figure></div>"
  rows+="<p class=camera>camera: <a href=\"original/$stem.json\">$stem.json</a></p></section>"
  [[ -f "$original" ]] || echo "$stem: no original picture $original" >&2
  player_files_check "$stem" || status=1
done
commit="$(git rev-parse --short HEAD)"
cat >"$dir/index.html" <<EOF
<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Look comparison</title>
<style>
:root { --bg: #fff; --fg: #1d1d1f; --muted: #6e6e73; }
@media (prefers-color-scheme: dark) { :root { --bg: #161618; --fg: #f2f2f2; --muted: #a1a1a6; } }
body { background: var(--bg); color: var(--fg); font: 15px/1.4 system-ui, sans-serif; margin: 16px; }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
figure { margin: 0; } img { width: 100%; height: auto; display: block; }
figcaption, .camera, p { color: var(--muted); }
section { margin-bottom: 32px; }
@media (max-width: 700px) { .pair { grid-template-columns: 1fr; } }
</style>
<h1>Look comparison</h1>
<p>The original (left) and s2-reborn $commit (right) at the same trace tick with the original's camera (TOOL-trace TRACE-8). Made by scripts/look-compare.sh.</p>
$rows
EOF
echo "page: $dir/index.html"
exit $status
