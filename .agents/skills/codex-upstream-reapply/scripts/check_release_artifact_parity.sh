#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
workflow="${repo_root}/.github/workflows/rust-release.yml"

if [[ ! -f "${workflow}" || ! -f "${repo_root}/codex-cli/package.json" ]]; then
  exit 0
fi

required_patterns=(
  "cargo build"
  "actions/upload-artifact"
  "npm publish"
)
for pattern in "${required_patterns[@]}"; do
  if ! rg -F --quiet -- "${pattern}" "${workflow}"; then
    echo "[ERROR] Release workflow is missing required release surface: ${pattern}" >&2
    exit 1
  fi
done

for path in \
  "${repo_root}/codex-cli/scripts/build_npm_package.py" \
  "${repo_root}/codex-cli/scripts/install_native_deps.py" \
  "${repo_root}/scripts/install/install.sh" \
  "${repo_root}/scripts/install/install.ps1" \
  "${repo_root}/scripts/codex_package/cargo.py"; do
  if [[ ! -f "${path}" ]]; then
    echo "[ERROR] Release packaging path is missing: ${path}" >&2
    exit 1
  fi
done

# Package-layout contract: the app-server daemon seeds its managed install from
# the vendored package root, so npm platform packages and release archives must
# ship the canonical codex-package tree (bin/, codex-path/, codex-resources/,
# codex-package.json). A layout regression breaks daemon startup on fresh
# installs with "this CLI has no complete local package".
launcher="${repo_root}/codex-cli/bin/codex.js"
installer="${repo_root}/codex-cli/scripts/install_native_deps.py"
npm_builder="${repo_root}/codex-cli/scripts/build_npm_package.py"

expect() {
  local pattern="$1" path="$2" label="$3"
  if ! rg -F --quiet -- "${pattern}" "${path}"; then
    echo "[ERROR] ${label}: expected pattern missing in ${path}: ${pattern}" >&2
    exit 1
  fi
}

reject() {
  local pattern="$1" path="$2" label="$3"
  if rg -F --quiet -- "${pattern}" "${path}"; then
    echo "[ERROR] ${label}: forbidden legacy layout pattern found in ${path}: ${pattern}" >&2
    exit 1
  fi
}

# codex.js must exec vendor/<target>/bin/codex and prepend codex-path to PATH.
expect '"bin"' "${launcher}" "launcher entrypoint"
expect '"codex-path"' "${launcher}" "launcher PATH dir"
reject 'archRoot, "codex"' "${launcher}" "launcher legacy codex/ dir"
reject 'archRoot, "path"' "${launcher}" "launcher legacy path/ dir"

# Native staging must place binaries/resources under the canonical dirs and
# emit codex-package.json manifests.
expect 'dest_dir="bin"' "${installer}" "installer bin dest"
expect '"codex-path"' "${installer}" "installer codex-path dest"
expect '"codex-resources"' "${installer}" "installer codex-resources dest"
expect 'codex-package.json' "${installer}" "installer package manifest"

# npm platform packages must consume the whole codex-package tree.
expect 'codex-package' "${npm_builder}" "npm codex-package component"

# The release workflow must build the canonical package and embed the bwrap
# digest for Linux targets.
expect '--bin bwrap' "${workflow}" "bwrap build"
expect 'CODEX_BWRAP_SHA256' "${workflow}" "bwrap digest"
expect 'build_codex_package.py' "${workflow}" "canonical package builder"

echo "[OK] Release CI structural preflight passed; complete semantic review from release-ci-sync.md"
