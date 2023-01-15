#!/bin/bash
set -Exeuo pipefail

print_status() {
  local level="$1"
  local body="${2//%/%25}"
  body="${body//$'\r'/}"
  body="${body//$'\n'/%0A}"

  echo "::$level::$body"
}

LASTVER="$(git tag | grep libuv-v | sort -V | tail -n1 | sed -e 's/^libuv-v//')"
print_status notice "latest libuv-sys: $LASTVER"

VER="$(curl https://api.github.com/repos/libuv/libuv/tags | jq -r --arg current "$LASTVER" 'def ver($v): $v | ltrimstr("v") | split(".") | map(tonumber); map(.name) | map(select(. | test("^v\\d+\\.\\d+\\.\\d+$";"s"))) | map(ver(.)) | map(select(. > ver($current))) | sort | first | if . == null then "" else join(".") end')"
if [ -z "$VER" ]; then
  print_status notice "no new libuv version"
  exit 0
fi
print_status notice "next libuv: $VER"
echo "version=v$VER" >> $GITHUB_OUTPUT
