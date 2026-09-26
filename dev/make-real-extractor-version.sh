#!/bin/sh
# Makes a Development version folder that uses the real randcrw-extract from the game repo, and
# the mock runtime from dev/mock unless a real runtime binary is given.
# usage: dev/make-real-extractor-version.sh [path/to/randcrw-extract] [out dir] [path/to/runtime]
set -e
here=$(cd "$(dirname "$0")" && pwd)
ex=${1:-$HOME/Repos/randcre/target/release/randcrw-extract}
out=${2:-$here/real}
[ -x "$ex" ] || { echo "no extractor at $ex"; exit 1; }
mkdir -p "$out"
ln -sf "$ex" "$out/randcrw-extract"
ln -sf "${3:-$here/mock/randcrw}" "$out/randcrw"
cat > "$out/randcrw-manifest.json" <<JSON
{"schema":1,"name":"randcrw","version":"0.1.0-realx","game":"rac1","runtime":"randcrw","extractor":"randcrw-extract","supported_discs":["SCUS_971.99"],"data_format":1}
JSON
echo "version folder: $out (add it under Settings → Version Management → Development)"
