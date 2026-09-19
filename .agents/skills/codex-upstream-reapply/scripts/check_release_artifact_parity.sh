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

echo "[OK] Release CI structural preflight passed; complete semantic review from release-ci-sync.md"
