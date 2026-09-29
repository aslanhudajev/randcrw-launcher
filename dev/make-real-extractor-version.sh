#!/bin/sh
# Makes a Development version folder that uses the real rerac-extract from the game repo, and
# the mock runtime from dev/mock unless a real runtime binary is given.
# usage: dev/make-real-extractor-version.sh [path/to/rerac-extract] [out dir] [path/to/runtime]
# RERAC_GAME_DIR is your local checkout of re-rac/rerac (default: the sibling folder ../randcre).
set -e
here=$(cd "$(dirname "$0")" && pwd)
game=${RERAC_GAME_DIR:-$here/../../randcre}
ex=${1:-$game/target/release/rerac-extract}
out=${2:-$here/real}
[ -x "$ex" ] || { echo "no extractor at $ex"; exit 1; }
mkdir -p "$out"
ln -sf "$ex" "$out/rerac-extract"
ln -sf "${3:-$here/mock/rerac}" "$out/rerac"
cat > "$out/rerac-manifest.json" <<JSON
{"schema":1,"name":"rerac","version":"0.1.0-realx","game":"rac1","runtime":"rerac","extractor":"rerac-extract","supported_discs":["SCUS_971.99"],"data_format":1}
JSON
echo "version folder: $out (add it under Settings → Version Management → Development)"
