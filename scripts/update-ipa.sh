#!/usr/bin/env bash
set -euo pipefail

repo="opa334/Dopamine"
mode="${1:-update}"
data_dir="${DOPAMINE_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/dopamine-sideload}"
target="$data_dir/Dopamine.ipa"
version_file="$data_dir/Dopamine.version"
tmp="$target.tmp.$$"
json_tmp="$tmp.json"
trap 'rm -f -- "$tmp" "$json_tmp"' EXIT

mkdir -p -- "$data_dir"
api="https://api.github.com/repos/$repo/releases/latest"
curl -fsSL --retry 3 --connect-timeout 10 -H 'Accept: application/vnd.github+json' "$api" -o "$json_tmp"
version=$(jq -er '.tag_name | strings | select(length > 0)' "$json_tmp")
url=$(jq -er '[.assets[] | select(.name == "Dopamine.ipa")][0].browser_download_url | strings | select(startswith("https://github.com/"))' "$json_tmp")
digest=$(jq -r '[.assets[] | select(.name == "Dopamine.ipa")][0].digest // empty' "$json_tmp")
local_version=""
if [[ -f "$version_file" ]]; then local_version=$(tr -d '[:space:]' < "$version_file"); fi
if [[ "$version" == "$local_version" && -s "$target" ]]; then
    if [[ "$mode" == "--check" ]]; then
        printf 'AKTUALNA|%s\n' "$version"
    else
        echo "Dopamine $version jest już pobrane."
    fi
    exit 0
fi

if [[ "$mode" == "--check" ]]; then
    if [[ -s "$target" ]]; then
        printf 'DOSTEPNA|%s|%s\n' "${local_version:-nieznana}" "$version"
    else
        printf 'BRAK|%s\n' "$version"
    fi
    exit 0
fi

echo "Pobieram Dopamine $version..."
curl -fL --retry 3 --connect-timeout 10 --progress-bar "$url" -o "$tmp"
[[ -s "$tmp" ]] || { echo "Pobrany IPA jest pusty." >&2; exit 1; }
python3 - "$tmp" <<'PY'
import sys, zipfile
p = sys.argv[1]
try:
    with zipfile.ZipFile(p) as archive:
        names = archive.namelist()
        if not any(n.startswith("Payload/") and n.endswith(".app/Info.plist") for n in names):
            raise SystemExit("Archiwum nie zawiera aplikacji iOS w Payload/.")
        bad = archive.testzip()
        if bad:
            raise SystemExit(f"Uszkodzony wpis ZIP: {bad}")
except (OSError, zipfile.BadZipFile) as error:
    raise SystemExit(f"Pobrany plik nie jest poprawnym IPA: {error}")
PY
if [[ "$digest" == sha256:* ]]; then
    expected=${digest#sha256:}
    actual=$(sha256sum "$tmp" | cut -d ' ' -f1)
    [[ "$actual" == "$expected" ]] || { echo "Nie zgadza się SHA-256 assetu." >&2; exit 1; }
fi
mv -f -- "$tmp" "$target"
printf '%s\n' "$version" > "$version_file.tmp.$$"
mv -f -- "$version_file.tmp.$$" "$version_file"
echo "Dopamine zaktualizowane do $version."
