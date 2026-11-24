#!/usr/bin/env bash
set -euo pipefail

repository="${DOPAMINE_REPOSITORY:-}"
ref="${DOPAMINE_REF:-main}"
if [[ -z "$repository" ]]; then
    echo "Ustaw DOPAMINE_REPOSITORY=OWNER/REPO przed uruchomieniem instalatora." >&2
    echo "Przykład: DOPAMINE_REPOSITORY=twoj-login/dopamine-sideload bash install.sh" >&2
    exit 2
fi
[[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || { echo "Niepoprawny OWNER/REPO." >&2; exit 2; }
[[ "$ref" =~ ^[A-Za-z0-9._/-]+$ && "$ref" != *..* && "$ref" != /* && "$ref" != */ ]] || {
    echo "Niepoprawna nazwa gałęzi." >&2
    exit 2
}

tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
archive="$tmp/source.tar.gz"
curl -fL --retry 3 --connect-timeout 10 "https://codeload.github.com/$repository/tar.gz/refs/heads/$ref" -o "$archive"
tar -xzf "$archive" -C "$tmp"
source_dir=$(find "$tmp" -mindepth 1 -maxdepth 1 -type d -print -quit)
[[ -n "$source_dir" && -f "$source_dir/scripts/install.sh" ]] || { echo "Archiwum nie zawiera instalatora." >&2; exit 1; }
bash "$source_dir/scripts/install.sh"
