#!/bin/bash
set -Exeuo pipefail

print_status() {
  local level="$1"
  local body="${2//%/%25}"
  body="${body//$'\r'/}"
  body="${body//$'\n'/%0A}"

  echo "::$level::$body"
}

LASTVER="$(curl https://api.github.com/repos/bmatcuk/libuv-sys/git/matching-refs/tags/libuv-v | jq -r 'def ver($v): $v | ltrimstr("refs/tags/libuv-v") | split(".") | map(tonumber); map(ver(.ref)) | sort | last | join(".")')"
print_status notice "latest libuv-sys: $LASTVER"

VER="$(curl https://api.github.com/repos/libuv/libuv/tags | jq -r --arg current "$LASTVER" 'def ver($v): $v | ltrimstr("v") | split(".") | map(tonumber); map(ver(.name)) | map(select(. > ver($current))) | sort | first | if . == null then "" else join(".") end')"
if [ -z "$VER" ]; then
  print_status notice "no new libuv version"
  exit 0
fi
print_status notice "next libuv: $VER"
echo "::set-output name=version::v$VER"
