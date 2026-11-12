#!/usr/bin/env bash
# Packs the plugin into a zip that OpenDeck installs with Plugins → Install from file.
#
#   scripts/package.sh [BIN_DIR [OUT]]
#
# BIN_DIR holds one folder per target triple with the plugin's binary in it, the way CI collects
# them: BIN_DIR/x86_64-unknown-linux-gnu/footprints-deck,
# BIN_DIR/x86_64-pc-windows-msvc/footprints-deck.exe, … Without BIN_DIR the plugin is built for
# this machine only. The packed manifest keeps the platforms that have a binary, so OpenDeck on
# any other platform says the plugin is unsupported instead of failing to start it.
#
# OUT defaults to target/footprints-deck-<version>.zip. Needs cargo (without BIN_DIR), jq and zip.
set -euo pipefail

id=com.cimtunia.footprints-deck
bin=footprints-deck
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
out=${2:-$root/target/$bin-$version.zip}
mkdir -p "$(dirname "$out")"
out=$(cd "$(dirname "$out")" && pwd)/$(basename "$out")

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
plugin=$stage/$id.sdPlugin
mkdir -p "$plugin/bin"

if [[ $# -ge 1 ]]; then
    cp -R "$1"/. "$plugin/bin/"
else
    host=$(rustc -vV | sed -n 's/^host: //p')
    cargo build --release --manifest-path "$root/Cargo.toml"
    mkdir -p "$plugin/bin/$host"
    for file in "$bin" "$bin.exe"; do
        if [[ -f $root/target/release/$file ]]; then
            cp "$root/target/release/$file" "$plugin/bin/$host/"
        fi
    done
fi

# The triples that have a binary, as the manifest names it.
present=$(cd "$plugin" && jq -r '.CodePaths | to_entries[] | "\(.key)\t\(.value)"' "$root/manifest.json" |
    while IFS=$'\t' read -r triple path; do
        if [[ -f $path ]]; then echo "$triple"; fi
    done | jq -R . | jq -s .)
if [[ $present == "[]" ]]; then
    echo "No binary for any platform in the manifest's CodePaths" >&2
    exit 1
fi
jq --argjson present "$present" \
    '.CodePaths |= with_entries(select(.key as $t | $present | index($t)))' \
    "$root/manifest.json" > "$plugin/manifest.json"
chmod +x "$plugin"/bin/*/* 2>/dev/null || true

cp -R "$root/icons" "$root/property-inspector" "$plugin/"

rm -f "$out"
(cd "$stage" && zip -qr "$out" "$id.sdPlugin")
echo "Packed $(jq -r '.CodePaths | keys | join(", ")' "$plugin/manifest.json") into $out"
