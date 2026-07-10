#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
workflow="${repo_root}/.github/workflows/rust-release.yml"

if [[ ! -f "${workflow}" || ! -f "${repo_root}/codex-cli/package.json" ]]; then
  exit 0
fi

required_patterns=(
  "-p codex-code-mode-host"
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

protocol_manifest="${repo_root}/codex-rs/protocol/Cargo.toml"
for target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  if ! rg -F --quiet -- "[target.${target}.dependencies]" "${protocol_manifest}" || \
    ! rg -F --quiet -- 'openssl-sys = { workspace = true, features = ["vendored"] }' "${protocol_manifest}"; then
    echo "[ERROR] Musl release dependency parity is missing vendored OpenSSL for ${target}" >&2
    exit 1
  fi
done

echo "[OK] Release artifact parity includes codex-code-mode-host"
