#!/usr/bin/env bash
set -euo pipefail
TAG="$1"
ARTIFACT_DIR="$2"
mapfile -d '' -t files < <(find "$ARTIFACT_DIR" -type f \( -name '*.dmg' -o -name '*-setup.exe' \) -print0)
if [ "${#files[@]}" -ne 3 ]; then
  echo "Expected three installers, found ${#files[@]}" >&2
  exit 1
fi
if ! gh release view "$TAG" >/dev/null 2>&1; then
  gh release create "$TAG" --draft --verify-tag --title "Hello Gitty $TAG" --generate-notes
fi
gh release upload "$TAG" "${files[@]}" --clobber
gh release edit "$TAG" --draft=false
