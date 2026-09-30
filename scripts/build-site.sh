#!/usr/bin/env bash
# Assemble the GitHub Pages site into _site/:
#   _site/index.html       the landing page, docs/index.html
#   _site/site.css         the shared stylesheet, docs/site.css
#   _site/docs/index.html  README.md and CHANGELOG.md rendered by microdocs
#                          with the docs/template.html template
# The __VERSION__ placeholder in both pages is replaced with the version
# from Cargo.toml.
#
# Used by `just site` locally and by .github/workflows/deploy-docs.yml.
# Requires uv (https://docs.astral.sh/uv/) for `uvx`.
set -euo pipefail
cd "$(dirname "$0")/.."

out="_site"
version=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

rm -rf "$out"
mkdir -p "$out/docs"

uvx --from microdocs microdocs README.md CHANGELOG.md \
    --title "Siteprobe" \
    --repo-url "https://github.com/bartTC/siteprobe" \
    --template docs/template.html \
    --output "$out/docs/index.html"

sed "s/__VERSION__/$version/g" docs/index.html > "$out/index.html"
sed "s/__VERSION__/$version/g" "$out/docs/index.html" > "$out/docs/index.html.tmp"
mv "$out/docs/index.html.tmp" "$out/docs/index.html"
cp docs/site.css "$out/site.css"

echo "Site built in $out/ (siteprobe v$version)"
