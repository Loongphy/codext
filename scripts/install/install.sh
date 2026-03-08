#!/bin/sh

set -eu

VERSION="${1:-latest}"
PACKAGE="@loongphy/codex"

step() {
  printf '==> %s\n' "$1"
}

normalize_version() {
  case "$1" in
    "" | latest)
      printf 'latest\n'
      ;;
    rust-v*)
      printf '%s\n' "${1#rust-v}"
      ;;
    v*)
      printf '%s\n' "${1#v}"
      ;;
    *)
      printf '%s\n' "$1"
      ;;
  esac
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required to install Codext." >&2
    exit 1
  fi
}

require_command npm

resolved_version="$(normalize_version "$VERSION")"
package_spec="$PACKAGE"
if [ "$resolved_version" != "latest" ]; then
  package_spec="$PACKAGE@$resolved_version"
fi

step "Installing Codext from npm"
npm install -g "$package_spec"

step "Run: codext"
