#!/bin/sh
# Checks that a release can be made of the code: the published crates at the same version, its
# changelog in changelogs/<version>.md, and the tag, when given, being this version.
#
#   scripts/check-release.sh            # in the CI, on every change
#   scripts/check-release.sh 0.2.0      # on the tag 0.2.0
set -eu

fail() {
  echo "check-release: $1" >&2
  exit 1
}

version_of() {
  # path+file:///…/crates/kraken-spot-api#0.1.0, or …#name@0.1.0 when the folder has another name
  cargo pkgid -p "$1" | sed 's/.*[#@]//'
}

version=$(version_of kraken-spot-api)
for crate in kraken-crypto kraken-network; do
  [ "$(version_of "$crate")" = "$version" ] ||
    fail "$crate is $(version_of "$crate"), kraken-spot-api $version: they're released together"
done

changelog="changelogs/$version.md"
[ -s "$changelog" ] || fail "$changelog is missing: the notes of the release $version"

if [ $# -gt 0 ]; then
  [ "$1" = "$version" ] || fail "the tag $1 isn't the version of the code, $version"
fi
echo "check-release: $version, $changelog"
