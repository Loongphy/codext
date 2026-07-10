#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
workflow="${repo_root}/.github/workflows/rust-release.yml"

if [[ ! -f "${workflow}" || ! -f "${repo_root}/codex-cli/package.json" ]]; then
  exit 0
fi

required_patterns=(
  "--bin codex-code-mode-host"
  "codex-code-mode-host"
  "codex-bin-"
  "CARGO_NET_GIT_FETCH_WITH_CLI"
)
for pattern in "${required_patterns[@]}"; do
  if ! rg -F --quiet -- "${pattern}" "${workflow}"; then
    echo "[ERROR] Release workflow is missing required native component: ${pattern}" >&2
    exit 1
  fi
done

for path in \
  "${repo_root}/codex-cli/scripts/build_npm_package.py" \
  "${repo_root}/codex-cli/scripts/install_native_deps.py" \
  "${repo_root}/scripts/install/install.sh" \
  "${repo_root}/scripts/install/install.ps1" \
  "${repo_root}/scripts/codex_package/cargo.py"; do
  if ! rg -F --quiet -- "codex-code-mode-host" "${path}"; then
    echo "[ERROR] Release packaging path is missing codex-code-mode-host: ${path}" >&2
    exit 1
  fi
done

if rg -F --quiet -- "-p codex-code-mode-host" "${workflow}"; then
  echo "[ERROR] Release workflow must build codex-code-mode-host in the same Cargo invocation as the CLI" >&2
  exit 1
fi

echo "[OK] Release artifact parity includes codex-code-mode-host"
