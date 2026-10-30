#!/bin/sh
# rice fork: refreshes the bundled snapshot of Zen Internet's site styles
# (sameerasw/my-internet, MIT). Run by hand; the app also updates itself.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/desktop/resources/site-styles"
mkdir -p "$out"
curl -sfL --max-filesize 4194304 https://sameerasw.github.io/my-internet/styles.json -o "$out/styles.json.tmp"
python3 -I -c "import json,sys; d=json.load(open(sys.argv[1])); assert 1 <= len(d['website']) <= 5000" "$out/styles.json.tmp"
gzip -9 -n -c "$out/styles.json.tmp" > "$out/styles.json.gz"
rm "$out/styles.json.tmp"
curl -sfL https://raw.githubusercontent.com/sameerasw/my-internet/main/LICENSE -o "$out/LICENSE-my-internet.txt"
