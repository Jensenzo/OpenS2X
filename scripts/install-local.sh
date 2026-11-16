#!/usr/bin/env bash
# Installs the dist build of OpenS2X.exe for double-click play on this
# Windows machine, from WSL:
#   - the exe to   %USERPROFILE%\Games\s2-reborn\OpenS2X.exe (the folder
#                  keeps the repository's name: s2-frontier's install takes
#                  the films from it); an s2-reborn.exe from before the
#                  public name (13 Nov 2026) is removed
#   - the game to  %USERPROFILE%\Games\S2-data\   (copied from $S2_GAME_DIR
#                  or ~/s2/game, only on first run or with --data)
#   - game_dir in  %APPDATA%\s2-reborn\settings.txt, other lines kept (the
#                  file is not touched when it names that folder already)
#   - a desktop shortcut "OpenS2X"; the old "s2-reborn" one is removed
#   - the films the program shows, converted from the game's own Bink files
#     with FFmpeg into %USERPROFILE%\Games\s2-reborn\video\ (docs/video.md;
#     once, or again with --video)
# Run it after each `cargo build --profile dist --target x86_64-pc-windows-gnu
# -p s2-viewer --bin OpenS2X`; pass --build to do that build first.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
source "$repo/scripts/recording-guard.sh"
refuse_while_recording
src_game="${S2_GAME_DIR:-$HOME/s2/game}"
build=0
data=0
video=0
for a in "$@"; do
    case "$a" in
        --build) build=1 ;;
        --data) data=1 ;;
        --video) video=1 ;;
        *) echo "usage: $0 [--build] [--data] [--video]" >&2; exit 2 ;;
    esac
done

if [ "$build" = 1 ]; then
    (cd "$repo" && cargo build --profile dist --target x86_64-pc-windows-gnu -p s2-viewer --bin OpenS2X)
fi
exe="$repo/target/x86_64-pc-windows-gnu/dist/OpenS2X.exe"
[ -f "$exe" ] || { echo "no $exe; build it or pass --build" >&2; exit 1; }

winpath() { powershell.exe -NoProfile -Command "$1" | tr -d '\r'; }
profile_w="$(winpath '[Environment]::GetFolderPath("UserProfile")')"
appdata_w="$(winpath '[Environment]::GetFolderPath("ApplicationData")')"
desktop_w="$(winpath '[Environment]::GetFolderPath("Desktop")')"
games="$(wslpath -u "$profile_w")/Games"
install="$games/s2-reborn"
gamedata="$games/S2-data"
install_w="$profile_w\\Games\\s2-reborn"
gamedata_w="$profile_w\\Games\\S2-data"

mkdir -p "$install"
cp -f "$exe" "$install/OpenS2X.exe"
cp -f "$repo/PLAYING.md" "$install/PLAYING.md"
echo "exe:      $install_w\\OpenS2X.exe"
# The program before its public name: gone, so that nothing starts an old
# build by it.
if [ -f "$install/s2-reborn.exe" ]; then
    rm -f "$install/s2-reborn.exe"
    echo "removed:  $install_w\\s2-reborn.exe (the game is OpenS2X.exe now)"
fi

if [ "$data" = 1 ] || [ ! -d "$gamedata/data" ]; then
    [ -d "$src_game/data" ] || { echo "no game data in $src_game" >&2; exit 1; }
    mkdir -p "$gamedata"
    # Every file again, not only newer ones (-u): a copy whose file is newer
    # but differs would otherwise stay as it is.
    cp -r "$src_game/." "$gamedata/"
fi
echo "game:     $gamedata_w"

# The films, from the game copy's own Bink files.
if [ "$video" = 1 ]; then
    "$repo/scripts/convert-films.sh" --force "$gamedata" "$install/video"
else
    "$repo/scripts/convert-films.sh" "$gamedata" "$install/video"
fi
echo "films:    $install_w\\video"

# game_dir in the settings file; every other line stays as it is.
# A file that names this folder already is left as it is, not written again
# with the same bytes (the player's file: nothing of ours rewrites it).
settings="$(wslpath -u "$appdata_w")/s2-reborn/settings.txt"
if [ -f "$settings" ] && tr -d '\r' <"$settings" | grep -qxF "game_dir=$gamedata_w"; then
    echo "settings: $appdata_w\\s2-reborn\\settings.txt (game_dir already this folder; left as it is)"
else
    mkdir -p "$(dirname "$settings")"
    touch "$settings"
    grep -v '^game_dir=' "$settings" > "$settings.new" || true
    { echo "game_dir=$gamedata_w"; cat "$settings.new"; } > "$settings"
    rm -f "$settings.new"
    echo "settings: $appdata_w\\s2-reborn\\settings.txt"
fi

powershell.exe -NoProfile -Command "
\$s = (New-Object -ComObject WScript.Shell).CreateShortcut('$desktop_w\\OpenS2X.lnk');
\$s.TargetPath = '$install_w\\OpenS2X.exe';
\$s.WorkingDirectory = '$install_w';
\$s.IconLocation = '$install_w\\OpenS2X.exe,0';
\$s.Description = 'OpenS2X';
\$s.Save()" >/dev/null
echo "shortcut: $desktop_w\\OpenS2X.lnk"
# The shortcut from before the public name pointed at s2-reborn.exe, which
# is gone: removed, OpenS2X.lnk takes its place.
old_link="$(wslpath -u "$desktop_w")/s2-reborn.lnk"
if [ -f "$old_link" ]; then
    rm -f "$old_link"
    echo "removed:  $desktop_w\\s2-reborn.lnk (use the OpenS2X shortcut)"
fi
