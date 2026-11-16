#!/usr/bin/env bash
# The Windows download of a release (docs/releasing.md, "The Windows download"):
#
#   scripts/dist.sh 0.9.0
#
# builds OpenS2X.exe with the dist profile for x86_64-pc-windows-gnu and
# packs target/dist/OpenS2X-<version>-windows-x64.zip with the program, its
# documents (the README without its pictures, scripts/zip-readme.py) and the
# third-party notices (scripts/third-party-licenses.py).
# Nothing from the original game goes in: the archive is made from a fixed
# list of files, and its listing is checked against that list afterwards.
set -euo pipefail
cd "$(dirname "$0")/.."
version="${1:?usage: scripts/dist.sh X.Y.Z}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $version"; exit 2; }
grep -q "^version = \"$version\"" Cargo.toml || { echo "Cargo.toml is not at $version"; exit 2; }

cargo build --profile dist --target x86_64-pc-windows-gnu -p s2-viewer --bin OpenS2X

# The program says which version it is in its Windows file properties
# (crates/viewer/build.rs; docs/releasing.md, "The version").
python3 - target/x86_64-pc-windows-gnu/dist/OpenS2X.exe "$version" <<'EOF'
import sys
data = open(sys.argv[1], "rb").read()
key = "ProductVersion\0".encode("utf-16-le")
at = data.find(key)
value = (sys.argv[2] + "\0").encode("utf-16-le")
if at < 0 or data.find(value, at + len(key), at + len(key) + 4 + len(value)) < 0:
    sys.exit(f"OpenS2X.exe has no version resource saying {sys.argv[2]} (is MinGW's windres installed?)")
print(f"OpenS2X.exe: ProductVersion {sys.argv[2]}")
EOF

name="OpenS2X-$version-windows-x64"
out="target/dist/$name"
rm -rf "$out" "target/dist/$name.zip"
mkdir -p "$out"
cp target/x86_64-pc-windows-gnu/dist/OpenS2X.exe "$out/"
cp PLAYING.md CHANGELOG.md LICENSE "$out/"
# The README without the pictures and badges only GitHub shows.
python3 -B scripts/zip-readme.py README.md > "$out/README.md"
python3 scripts/third-party-licenses.py > "$out/THIRD-PARTY-LICENSES.txt"

python3 - "$out" "target/dist/$name.zip" <<'EOF'
import pathlib, sys, zipfile
src, dst = pathlib.Path(sys.argv[1]), sys.argv[2]
with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as z:
    for f in sorted(src.iterdir()):
        z.write(f, f"{src.name}/{f.name}")
EOF

# The listing must be exactly these files.
expected="CHANGELOG.md LICENSE OpenS2X.exe PLAYING.md README.md THIRD-PARTY-LICENSES.txt"
listed="$(python3 -c 'import sys, zipfile; print(" ".join(sorted(n.split("/", 1)[1] for n in zipfile.ZipFile(sys.argv[1]).namelist())))' "target/dist/$name.zip")"
if [ "$listed" != "$expected" ]; then
    echo "unexpected archive contents: $listed" >&2
    exit 1
fi
python3 -c 'import sys, zipfile
for i in zipfile.ZipFile(sys.argv[1]).infolist(): print(f"{i.file_size:>12}  {i.filename}")' "target/dist/$name.zip"
sha256sum "target/dist/$name.zip"
