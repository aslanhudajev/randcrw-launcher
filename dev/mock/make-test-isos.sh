#!/bin/sh
# Creates empty placeholder disc images that steer the mock extractor.
# usage: dev/mock/make-test-isos.sh <dir>
set -e
d="${1:-.}"
mkdir -p "$d"
for f in "Ratchet & Clank (USA).iso" "Ratchet & Clank (Europe).iso" "notrac-err20.iso" "disk-full-err31.iso" "bad-copy-err40.iso"; do
  : > "$d/$f"
done
echo "created test images in $d"
