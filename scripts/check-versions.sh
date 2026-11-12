#!/usr/bin/env bash
# Checks that every file naming the release version names the same one, the version in
# src-tauri/tauri.conf.json. The app and the Stream Deck plugin are released together, so they
# share it.
#
#   scripts/check-versions.sh [VERSION]
#
# With VERSION, every file must name that one instead. Exits non-zero, listing the files that
# differ. Needs only sed and grep.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)

# The first `"version": "…"` of a JSON file, the top-level one in the files checked here.
json_version() {
    sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

# The first `version = "…"` of a Cargo.toml, the one of its [package].
cargo_version() {
    sed -n 's/^version = "\(.*\)"$/\1/p' "$1" | head -n 1
}

# The version Cargo.lock records for package `$2`.
lock_version() {
    grep -A1 -x "name = \"$2\"" "$1" | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1
}

expected=${1:-$(json_version "$root/src-tauri/tauri.conf.json")}

found=(
    "src-tauri/tauri.conf.json=$(json_version "$root/src-tauri/tauri.conf.json")"
    "package.json=$(json_version "$root/package.json")"
    "src-tauri/Cargo.toml=$(cargo_version "$root/src-tauri/Cargo.toml")"
    "src-tauri/Cargo.lock=$(lock_version "$root/src-tauri/Cargo.lock" footprint)"
    "stream-deck-plugin/Cargo.toml=$(cargo_version "$root/stream-deck-plugin/Cargo.toml")"
    "stream-deck-plugin/Cargo.lock=$(lock_version "$root/stream-deck-plugin/Cargo.lock" footprints-deck)"
)

status=0
for entry in "${found[@]}"; do
    file=${entry%%=*}
    version=${entry#*=}
    if [[ $version != "$expected" ]]; then
        echo "$file has version ${version:-<none>}, expected $expected" >&2
        status=1
    fi
done
if [[ $status -eq 0 ]]; then
    echo "Every file has version $expected"
fi
exit $status
