#!/bin/bash
set -Exeuo pipefail

print_status() {
  local level="$1"
  local body="${2//%/%25}"
  body="${body//$'\r'/}"
  body="${body//$'\n'/%0A}"

  echo "::$level::$body"
}

# setup git
git config --local user.name $GIT_USER_NAME
git config --local user.email $GIT_USER_EMAIL

# make sure to fetch tags
git fetch --tags
pushd libuv
git fetch --tags
popd

# get version info
OUR_TAG=$(grep '^version =' Cargo.toml | awk -F'"' '{print "v" $2}')
PREV_TAG=$({ echo $OUR_TAG; git tag; } | sort -V | grep -B1 $OUR_TAG | head -n 1)
LIBUV_VERSION=$({ cd libuv; git describe --tags; })
LIBUV_TAG="libuv-${LIBUV_VERSION}"

# set tags
git tag $LIBUV_TAG || true
git tag -a $OUR_TAG -m "Corresponds to libuv ${LIBUV_VERSION}"
git push --tags


# build RELEASELOG.md and release
{ echo -e "Corresponds to libuv ${LIBUV_VERSION}.\n\n## Changelog\n\n"; git log --pretty=format:"%h %s" "${PREV_TAG}.." 2>/dev/null; } > RELEASELOG.md || true
gh release create "$OUR_TAG" README.md --notes-file RELEASELOG.md --title "$OUR_TAG"

MSG="libuv-sys $OUR_TAG published"
print_status notice "$MSG"
