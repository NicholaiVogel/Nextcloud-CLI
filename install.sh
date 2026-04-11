#!/usr/bin/env sh
set -eu

REPO="${NEXTCLOUD_CLI_INSTALL_REPO:-NicholaiVogel/Nextcloud-CLI}"
VERSION="${NEXTCLOUD_CLI_INSTALL_VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-${NEXTCLOUD_CLI_INSTALL_DIR:-$HOME/.local/bin}}"

log() {
  printf '%s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

detect_os() {
  if [ -n "${NEXTCLOUD_CLI_INSTALL_OS:-}" ]; then
    printf '%s\n' "$NEXTCLOUD_CLI_INSTALL_OS"
    return
  fi

  case "$(uname -s)" in
    Darwin) printf 'darwin\n' ;;
    Linux) printf 'linux\n' ;;
    MINGW* | MSYS* | CYGWIN*) printf 'win32\n' ;;
    *) die "unsupported operating system: $(uname -s)" ;;
  esac
}

detect_arch() {
  if [ -n "${NEXTCLOUD_CLI_INSTALL_ARCH:-}" ]; then
    printf '%s\n' "$NEXTCLOUD_CLI_INSTALL_ARCH"
    return
  fi

  case "$(uname -m)" in
    x86_64 | amd64) printf 'x64\n' ;;
    arm64 | aarch64) printf 'arm64\n' ;;
    *) die "unsupported architecture: $(uname -m)" ;;
  esac
}

platform_triple() {
  os="$1"
  arch="$2"

  case "${os}/${arch}" in
    linux/x64) printf 'x86_64-unknown-linux-gnu\n' ;;
    darwin/x64) printf 'x86_64-apple-darwin\n' ;;
    darwin/arm64) printf 'aarch64-apple-darwin\n' ;;
    win32/x64) printf 'x86_64-pc-windows-msvc\n' ;;
    *)
      die "unsupported platform: ${os}/${arch}. Supported installer platforms are linux/x64, darwin/x64, darwin/arm64, and win32/x64."
      ;;
  esac
}

resolve_latest_version() {
  latest_url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest")"
  tag="${latest_url##*/}"
  tag="${tag#v}"

  case "$tag" in
    '' | latest) die "could not resolve latest release for ${REPO}" ;;
    *) printf '%s\n' "$tag" ;;
  esac
}

download() {
  url="$1"
  destination="$2"
  curl -fsSL --retry 3 --retry-delay 1 --connect-timeout 15 -o "$destination" "$url"
}

verify_checksum() {
  archive="$1"
  checksum="$2"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$checksum"
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$checksum"
    return
  fi

  die "required command not found: sha256sum or shasum"
}

ensure_install_dir() {
  dir="$1"

  if [ -d "$dir" ]; then
    return
  fi

  if mkdir -p "$dir" 2>/dev/null; then
    return
  fi

  if command -v sudo >/dev/null 2>&1; then
    sudo mkdir -p "$dir"
    return
  fi

  die "could not create install directory: $dir"
}

install_file() {
  source="$1"
  destination="$2"
  dir="$(dirname "$destination")"

  if [ -w "$dir" ]; then
    if command -v install >/dev/null 2>&1; then
      install -m 0755 "$source" "$destination"
    else
      cp "$source" "$destination"
      chmod 0755 "$destination"
    fi
    return
  fi

  if command -v sudo >/dev/null 2>&1; then
    if command -v install >/dev/null 2>&1; then
      sudo install -m 0755 "$source" "$destination"
    else
      sudo cp "$source" "$destination"
      sudo chmod 0755 "$destination"
    fi
    return
  fi

  die "install directory is not writable and sudo is not available: $dir"
}

need_cmd curl
need_cmd tar

os="$(detect_os)"
arch="$(detect_arch)"
triple="$(platform_triple "$os" "$arch")"

if [ "$VERSION" = "latest" ]; then
  if [ -n "${NEXTCLOUD_CLI_RELEASE_BASE_URL:-}" ]; then
    die "NEXTCLOUD_CLI_INSTALL_VERSION must be set when NEXTCLOUD_CLI_RELEASE_BASE_URL is used"
  fi
  VERSION="$(resolve_latest_version)"
fi

VERSION="${VERSION#v}"
asset="nextcloud-cli-${VERSION}-${triple}.tar.gz"
base_url="${NEXTCLOUD_CLI_RELEASE_BASE_URL:-https://github.com/${REPO}/releases/download/v${VERSION}}"
tmp="$(mktemp -d)"

cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

log "Installing nextcloud-cli ${VERSION} for ${triple}"
log "Install directory: ${INSTALL_DIR}"

cd "$tmp"
download "${base_url}/${asset}.sha256" "${asset}.sha256"
download "${base_url}/${asset}" "$asset"
verify_checksum "$asset" "${asset}.sha256"

tar -xzf "$asset"
package_dir="nextcloud-cli-${VERSION}-${triple}"

case "$os" in
  win32)
    long_name="nextcloud-cli.exe"
    short_name="nxc.exe"
    ;;
  *)
    long_name="nextcloud-cli"
    short_name="nxc"
    ;;
esac

[ -f "${package_dir}/${long_name}" ] || die "release archive did not contain ${long_name}"
[ -f "${package_dir}/${short_name}" ] || die "release archive did not contain ${short_name}"

ensure_install_dir "$INSTALL_DIR"
install_file "${package_dir}/${long_name}" "${INSTALL_DIR}/${long_name}"
install_file "${package_dir}/${short_name}" "${INSTALL_DIR}/${short_name}"

log "Installed:"
log "  ${INSTALL_DIR}/${long_name}"
log "  ${INSTALL_DIR}/${short_name}"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) warn "${INSTALL_DIR} is not on PATH" ;;
esac

"${INSTALL_DIR}/${short_name}" --version
