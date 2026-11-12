#!/usr/bin/env bash
# Sets the release version of the app and the Stream Deck plugin, which are released together.
#
#   scripts/bump-version.sh 0.4.0
#
# Writes it into package.json, src-tauri/tauri.conf.json, both Cargo.toml and both Cargo.lock,
# then checks the result with scripts/check-versions.sh. Commit the change as
# `chore: version <VERSION>`. Needs sed and cargo.
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 VERSION" >&2
    exit 2
fi
version=$1
# Like the versions released so far: 0.3.0, or a pre-release such as 0.2.0-1.
if ! [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
    echo "$version is not a version like 0.4.0 or 0.4.0-1" >&2
    exit 2
fi

root=$(cd "$(dirname "$0")/.." && pwd)

# Sets the first version in `$1`: the top-level `"version": "…"` of a JSON file, or the
# `version = "…"` of a Cargo.toml's [package]. awk rather than `sed -i`, which differs between
# GNU and BSD sed.
set_version() {
    local file=$1 tmp
    tmp=$(mktemp)
    awk -v version="$version" '
        !done && /^[ \t]*"version":[ \t]*"/ {
            sub(/"version":[ \t]*"[^"]*"/, "\"version\": \"" version "\""); done = 1
        }
        !done && /^version = "/ { $0 = "version = \"" version "\""; done = 1 }
        { print }
        END { if (!done) exit 1 }' "$file" > "$tmp" || {
        rm -f "$tmp"
        echo "No version found in $file" >&2
        exit 1
    }
    cat "$tmp" > "$file"
    rm -f "$tmp"
}

for file in package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml stream-deck-plugin/Cargo.toml; do
    set_version "$root/$file"
done

# Records the new versions in the lockfiles; nothing else in them changes.
for crate in src-tauri stream-deck-plugin; do
    cargo update --workspace --quiet --manifest-path "$root/$crate/Cargo.toml"
done

"$root/scripts/check-versions.sh" "$version"
