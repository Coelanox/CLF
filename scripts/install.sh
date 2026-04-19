#!/usr/bin/env bash
set -euo pipefail

# Install the CLF CLI from GitHub release assets.
# Defaults:
#   - repository: Coelanox/CLF
#   - version: latest
#   - install dir: ~/.local/bin
#
# Environment overrides:
#   CLF_REPO=owner/repo
#   CLF_VERSION=latest|vX.Y.Z
#   CLF_INSTALL_DIR=/path/to/bin
#   CLF_ADD_TO_PATH=1  append a PATH line to ~/.profile if the dir is missing (opt-in)
#
# Usage:
#   bash scripts/install.sh
#   CLF_VERSION=v0.1.2 bash scripts/install.sh

CLF_REPO="${CLF_REPO:-Coelanox/CLF}"
CLF_VERSION="${CLF_VERSION:-latest}"
CLF_INSTALL_DIR="${CLF_INSTALL_DIR:-$HOME/.local/bin}"

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "error: missing required command: $1" >&2
        exit 1
    fi
}

require_cmd uname
require_cmd mktemp
require_cmd tar

if command -v curl >/dev/null 2>&1; then
    DOWNLOADER="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    DOWNLOADER="wget -qO-"
else
    echo "error: missing downloader (curl or wget)" >&2
    exit 1
fi

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
    *)
        echo "error: unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

ASSET="clf-${TARGET}.tar.gz"
if [[ "$CLF_VERSION" == "latest" ]]; then
    URL="https://github.com/${CLF_REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${CLF_REPO}/releases/download/${CLF_VERSION}/${ASSET}"
fi

TMP_DIR="$(mktemp -d)"
ARCHIVE_PATH="${TMP_DIR}/${ASSET}"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "Downloading ${URL}"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$ARCHIVE_PATH"
else
    wget -q "$URL" -O "$ARCHIVE_PATH"
fi

tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"

if [[ ! -f "${TMP_DIR}/clf" ]]; then
    echo "error: downloaded archive does not contain 'clf' binary" >&2
    exit 1
fi

mkdir -p "$CLF_INSTALL_DIR"
install -m 0755 "${TMP_DIR}/clf" "${CLF_INSTALL_DIR}/clf"

echo "Installed clf to ${CLF_INSTALL_DIR}/clf"

profile_line="export PATH=\"${CLF_INSTALL_DIR}:\$PATH\""
if [[ ":${PATH}:" == *":${CLF_INSTALL_DIR}:"* ]]; then
    echo "${CLF_INSTALL_DIR} is already on PATH for this session."
elif [[ "${CLF_ADD_TO_PATH:-}" == "1" ]]; then
    profile="${HOME}/.profile"
    if [[ -f "${profile}" ]] && grep -Fq "${CLF_INSTALL_DIR}" "${profile}"; then
        echo "PATH line for ${CLF_INSTALL_DIR} already present in ${profile}."
    else
        {
            echo ""
            echo "# Added by CLF install.sh — clf CLI"
            echo "${profile_line}"
        } >>"${profile}"
        echo "Appended PATH export to ${profile}. Run: source ${profile} or open a new shell."
    fi
else
    echo "Note: ${CLF_INSTALL_DIR} is not in PATH for this session."
    echo "Add this to your shell profile, or re-run with CLF_ADD_TO_PATH=1 to append to ~/.profile:"
    echo "  ${profile_line}"
fi
