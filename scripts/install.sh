#!/usr/bin/env sh
set -eu

REPO="Loongphy/codext"
INSTALL_DIR="$HOME/.local/bin"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}-${ARCH}" in
  Linux-x86_64) ASSET="codex-x86_64-unknown-linux-gnu.tar.gz" ;;
  Darwin-arm64 | Darwin-aarch64) ASSET="codex-aarch64-apple-darwin.tar.gz" ;;
  *) echo "Unsupported platform: ${OS}-${ARCH}" >&2 && exit 1 ;;
esac

URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
TMP_BASE="$(mktemp -d "${TMPDIR:-/tmp}/codex-install.XXXXXX")"
ARCHIVE_PATH="${TMP_BASE}/${ASSET}"

cleanup() {
  rm -rf "${TMP_BASE}"
}
trap cleanup EXIT INT TERM

echo "Installing latest Codex from ${REPO}"
echo "Downloading asset: ${ASSET}"
curl -fL "${URL}" -o "${ARCHIVE_PATH}"
tar -xzf "${ARCHIVE_PATH}" -C "${TMP_BASE}"
BIN_PATH="$(
  find "${TMP_BASE}" -type f \( -name codex -o -name 'codex-*' \) \
    | head -n 1
)"

if [ -z "${BIN_PATH}" ]; then
  echo "Failed to locate codex binary in archive ${ASSET}" >&2
  exit 1
fi

if [ ! -d "${INSTALL_DIR}" ]; then
  if mkdir -p "${INSTALL_DIR}" 2>/dev/null; then
    :
  elif command -v sudo >/dev/null 2>&1; then
    sudo mkdir -p "${INSTALL_DIR}"
  else
    echo "Cannot create install dir: ${INSTALL_DIR}" >&2
    exit 1
  fi
fi

if [ -w "${INSTALL_DIR}" ]; then
  install -m 0755 "${BIN_PATH}" "${INSTALL_DIR}/codex"
elif command -v sudo >/dev/null 2>&1; then
  sudo install -m 0755 "${BIN_PATH}" "${INSTALL_DIR}/codex"
else
  echo "No permission to write ${INSTALL_DIR}, and sudo is unavailable." >&2
  exit 1
fi

echo "Installed: ${INSTALL_DIR}/codex"
"${INSTALL_DIR}/codex" --version

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "Add to PATH if needed:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
