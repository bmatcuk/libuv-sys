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

# parse tag url
if [[ ! "$LIBUV_TAG_URL" =~ https://github.com/libuv/libuv/releases/tag/(v[0-9]+.[0-9]+.[0-9]+) ]]; then
  MSG="Unexpected tag url: $LIBUV_TAG_URL"
  print_status warning "$MSG"
  exit 1
fi

# setup some variables for libuv stuff
LIBUV_VERSION="${BASH_REMATCH[1]}"
LIBUV_MAJ_MIN="${LIBUV_VERSION%.*}"
LIBUV_PREV_VER="$(cd libuv && git tag | sort -V | grep -B1 "${LIBUV_VERSION}" | head -n1)"
LIBUV_CMAKE_CHANGES="$(cd libuv && git log --oneline "${LIBUV_PREV_VER}..${LIBUV_VERSION}" CMakeLists.txt)"
if [ -n "$LIBUV_CMAKE_CHANGES" ]; then
  MSG="Cannot automatically prepare a new build for libuv $LIBUV_VERSION because of changes to CMakeLists.txt:"$'\n'$'\n'"$LIBUV_CMAKE_CHANGES"
  print_status error "$MSG"
  exit 1
fi

echo "New libuv version: $LIBUV_VERSION"
echo "Previous libuv version: $LIBUV_PREV_VER"

# setup variables for libuv-sys stuff
LIBUV_SYS_BRANCH="${LIBUV_MAJ_MIN}.x"
LIBUV_SYS_PREV_VER=$(git describe "libuv-${LIBUV_PREV_VER}" 2>/dev/null || echo -n "")
if [ -z "$LIBUV_SYS_PREV_VER" ]; then
  MSG="Cannot automatically prepare a new build for libuv $LIBUV_VERSION because libuv-sys does not support libuv $LIBUV_PREV_VER"
  print_status error "$MSG"
  exit 1
fi

echo "libuv-sys branch: $LIBUV_SYS_BRANCH"

# calculate version number for libuv-sys
LIBUV_SYS_NEXT_VER="$(git tag | grep "${LIBUV_MAJ_MIN}." | sort -V | tail -n1)"
if [[ "$LIBUV_SYS_NEXT_VER" =~ .([0-9]+)$ ]]; then
  LIBUV_SYS_NEXT_VER="${LIBUV_MAJ_MIN}.$(( "${BASH_REMATCH[1]}" + 1 ))"
else
  LIBUV_SYS_NEXT_VER="${LIBUV_MAJ_MIN}.0"
fi

echo "New libuv-sys version: $LIBUV_SYS_NEXT_VER"

# checkout branch
if git branch -r | grep -q "${LIBUV_SYS_BRANCH}"; then
  git checkout --track "origin/$LIBUV_SYS_BRANCH"
else
  git checkout -b "$LIBUV_SYS_BRANCH" "$LIBUV_SYS_PREV_VER"
fi

# update version in Cargo.toml, build.rs
sed -i -e '/^version =/s/".*"/"'"${LIBUV_SYS_NEXT_VER#v}"'"/' Cargo.toml
sed -i -e '/^static LIBUV_VERSION:/s/".*"/"'"${LIBUV_VERSION#v}"'"/' build.rs

# update submodule
pushd libuv
git checkout "$LIBUV_VERSION"
popd

# commit
git add Cargo.toml build.rs libuv
git commit -m "preparing build for libuv $LIBUV_VERSION"
git push --all

MSG="libuv-sys $LIBUV_SYS_NEXT_VER prepared for libuv $LIBUV_VERSION"
print_status notice "$MSG"
