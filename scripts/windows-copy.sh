# Sourced by the scripts that run a Windows build from WSL (play-test.sh,
# golden-images.sh, view-map.sh, golden.sh): `windows_copy EXE` copies the
# built program to a name of this script's own and sets $copy to it; the
# copies are removed when the script exits.
#
# Windows runs a program from \\wsl.localhost and keeps its image cached
# while any process started from that path is still running. When cargo
# relinks the file meanwhile (another script, another session), the
# processes started afterwards get the old cached pages mixed with the new
# file's and crash anywhere: access violations, or a stack overflow from
# exceptions raised inside exception handling (the play harness's failure of
# 2026-10-20, while golden-images.sh ran shots; reproduced by replacing the
# file under a running viewer). A copy of its own is never replaced under a
# running process. It sits beside the original, so `video/` beside the
# viewer still resolves.
windows_copies=()

windows_copy() {
  copy="${1%.exe}.run-$$.exe"
  cp "$1" "$copy"
  windows_copies+=("$copy")
  trap 'rm -f "${windows_copies[@]}"' EXIT
}
