#!/bin/bash
set -Eeuo pipefail

send_email() {
  local body="$1"
  local subject="${2-"libuv-sys new version fail"}"

  curl -s -X POST \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -d "$(jo value1="$subject" value2="$body")" \
    "https://maker.ifttt.com/trigger/travis_status/with/key/$TRAVIS_WEBHOOK_KEY"
}

# setup git
git config --local user.name $GIT_USER_NAME
git config --local user.email $GIT_USER_EMAIL
git remote set-url origin https://${GITHUB_TOKEN}@github.com/bmatcuk/libuv-sys.git

# parse tag url
if [[ ! "$LIBUV_TAG_URL" =~ https://github.com/libuv/libuv/releases/tag/(v[0-9]+.[0-9]+.[0-9]+) ]]; then
  MSG="Unexpected tag url: $LIBUV_TAG_URL"
  send_email "$MSG"
  echo "$MSG"
  exit 1
fi

# setup some variables for libuv stuff
LIBUV_VERSION="${BASH_REMATCH[1]}"
LIBUV_MAJ_MIN="${LIBUV_VERSION%.*}"
LIBUV_PREV_VER="$(cd libuv && git tag | sort -V | grep -B1 "${LIBUV_VERSION}" | head -n1)"
LIBUV_CMAKE_CHANGES="$(cd libuv && git log --oneline "${LIBUV_PREV_VER}..${LIBUV_VERSION}" CMakeLists.txt)"
if [ -n "$LIBUV_CMAKE_CHANGES" ]; then
  MSG="Cannot automatically prepare a new build for libuv $LIBUV_VERSION because of changes to CMakeLists.txt:\n\n$LIBUV_CMAKE_CHANGES"
  send_email "$MSG"
  echo -e "$MSG"
  exit 0
fi

echo "New libuv version: $LIBUV_VERSION"
echo "Previous libuv version: $LIBUV_PREV_VER"

# setup variables for libuv-sys stuff
LIBUV_SYS_BRANCH="${LIBUV_MAJ_MIN}.x"
LIBUV_SYS_HAS_BRANCH="$(git branch | grep -q "$LIBUV_SYS_BRANCH" && echo "TRUE")"
LIBUV_SYS_PREV_VER=$(git describe "libuv-${LIBUV_PREV_VER}" 2>/dev/null || echo -n "")
if [ -z "$LIBUV_SYS_PREV_VER" ]; then
  MSG="Cannot automatically prepare a new build for libuv $LIBUV_VERSION because libuv-sys does not support libuv $LIBUV_PREV_VER"
  send_email "$MSG"
  echo "$MSG"
  exit 0
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
if git branch | grep -q "${LIBUV_SYS_BRANCH}"; then
  git checkout "$LIBUV_SYS_BRANCH"
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

send_email "libuv-sys $LIBUV_SYS_NEXT_VER prepared for libuv $LIBUV_VERSION" "libuv-sys new version!"
