#!/usr/bin/env bash
# Converts the original's films from the player's own game files into the
# form s2-reborn plays (docs/video.md), with FFmpeg:
#   <out>/<name>/frames.mjpeg   the frames as JPEG images, one after another
#   <out>/<name>/audio.wav      16-bit PCM, when the film has sound: the
#                               intro's track 0 mixed with the language's
#                               voice track (UI-MENU-0.3a)
#   <out>/<name>/info.txt       fps=, width=, height=, frames=
# Only the films the program shows: the main menu's town (title) and the
# intro; the company logo films are left out (UI-MENU-0.1).
#   scripts/convert-films.sh [--force] <game dir> <out dir>
# A film already converted is kept unless --force is given.
set -euo pipefail

force=0
if [ "${1:-}" = "--force" ]; then
    force=1
    shift
fi
[ $# = 2 ] || { echo "usage: $0 [--force] <game dir> <out dir>" >&2; exit 2; }
game="$1"
out="$2"

if ! command -v ffmpeg >/dev/null || ! command -v ffprobe >/dev/null; then
    echo "films: ffmpeg and ffprobe are needed (sudo apt install ffmpeg); skipped" >&2
    exit 0
fi

# UI-MENU-0.3a: the voice track of the game's language (menu.cfg, an rc00
# file; s2-app reads it). S2_APP may name a built s2-app; else cargo runs it
# from this repo. Without either: track 2, the shipped (English) game's.
repo="$(cd "$(dirname "$0")/.." && pwd)"
menu_cfg="$game/data/scripts/menu.cfg"
if [ -n "${S2_APP:-}" ]; then
    voice="$("$S2_APP" voice-track "$menu_cfg" 2>/dev/null | tr -d '\r')" || voice=""
elif command -v cargo >/dev/null && [ -f "$repo/Cargo.toml" ]; then
    voice="$(cd "$repo" && cargo run -q -p s2-app -- voice-track "$menu_cfg" 2>/dev/null)" || voice=""
fi
case "${voice:-}" in
    [0-9]) ;;
    *) echo "films: language of $menu_cfg not read; voice track 2" >&2; voice=2 ;;
esac

# name in the program : file in data/video
for pair in title:title.bik intro:Intro.bik; do
    name="${pair%%:*}"
    file="$game/data/video/${pair#*:}"
    dir="$out/$name"
    if [ ! -f "$file" ]; then
        echo "films: no $file" >&2
        continue
    fi
    if [ "$force" = 0 ] && [ -f "$dir/info.txt" ]; then
        echo "films: $name already converted"
        continue
    fi
    mkdir -p "$dir"
    rm -f "$dir/audio.wav" "$dir/info.txt"
    ffmpeg -v error -y -i "$file" -an -c:v mjpeg -q:v 4 -pix_fmt yuvj420p \
        -f mjpeg "$dir/frames.mjpeg"
    # UI-MENU-0.3a: a film with the four tracks 0 to 3 (the intro) plays
    # track 0 (music and effects) and the language's voice track, mixed at
    # one volume (summed); a film with fewer plays every track it has.
    ids=($(ffprobe -v error -select_streams a -show_entries stream=id \
        -of csv=p=0 "$file" | tr -d '\r'))
    if [ "${#ids[@]}" -gt 0 ]; then
        picks=()
        if [ "${#ids[@]}" -ge 4 ]; then
            for want in 0 "$voice"; do
                for k in "${!ids[@]}"; do
                    [ "$((ids[k]))" = "$want" ] && picks+=("$k")
                done
            done
        else
            picks=("${!ids[@]}")
        fi
        inputs=""
        for k in "${picks[@]}"; do inputs+="[0:a:$k]"; done
        ffmpeg -v error -y -i "$file" -filter_complex \
            "${inputs}amix=inputs=${#picks[@]}:normalize=0:duration=longest[a]" \
            -map "[a]" -c:a pcm_s16le "$dir/audio.wav"
        echo "films: $name sound: tracks ${picks[*]/#/a:} of ${#ids[@]} mixed"
    fi
    v() {
        ffprobe -v error -select_streams v:0 -count_packets \
            -show_entries "stream=$1" -of default=nw=1:nk=1 "$file"
    }
    {
        echo "fps=$(v r_frame_rate)"
        echo "width=$(v width)"
        echo "height=$(v height)"
        echo "frames=$(v nb_read_packets)"
    } > "$dir/info.txt"
    echo "films: $name -> $dir ($(tr '\n' ' ' < "$dir/info.txt"))"
done
