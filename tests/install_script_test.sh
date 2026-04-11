#!/usr/bin/env sh
set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
tmp="$(mktemp -d)"

cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

version="9.9.9"
triple="x86_64-unknown-linux-gnu"
package_dir="${tmp}/release/nextcloud-cli-${version}-${triple}"
install_dir="${tmp}/bin"

mkdir -p "$package_dir"

cat > "${package_dir}/nextcloud-cli" <<'EOF'
#!/usr/bin/env sh
printf 'nextcloud-cli 9.9.9\n'
EOF

cat > "${package_dir}/nxc" <<'EOF'
#!/usr/bin/env sh
printf 'nextcloud-cli 9.9.9\n'
EOF

chmod 0755 "${package_dir}/nextcloud-cli" "${package_dir}/nxc"
tar -C "${tmp}/release" -czf "${tmp}/release/nextcloud-cli-${version}-${triple}.tar.gz" "nextcloud-cli-${version}-${triple}"
(
  cd "${tmp}/release"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "nextcloud-cli-${version}-${triple}.tar.gz" > "nextcloud-cli-${version}-${triple}.tar.gz.sha256"
  else
    shasum -a 256 "nextcloud-cli-${version}-${triple}.tar.gz" > "nextcloud-cli-${version}-${triple}.tar.gz.sha256"
  fi
)

INSTALL_DIR="$install_dir" \
NEXTCLOUD_CLI_INSTALL_VERSION="$version" \
NEXTCLOUD_CLI_RELEASE_BASE_URL="file://${tmp}/release" \
NEXTCLOUD_CLI_INSTALL_OS="linux" \
NEXTCLOUD_CLI_INSTALL_ARCH="x64" \
  sh "${repo_root}/install.sh" >"${tmp}/install.log"

"${install_dir}/nxc" --version | grep 'nextcloud-cli 9.9.9' >/dev/null
"${install_dir}/nextcloud-cli" --version | grep 'nextcloud-cli 9.9.9' >/dev/null

if INSTALL_DIR="$install_dir" \
  NEXTCLOUD_CLI_INSTALL_VERSION="$version" \
  NEXTCLOUD_CLI_RELEASE_BASE_URL="file://${tmp}/release" \
  NEXTCLOUD_CLI_INSTALL_OS="linux" \
  NEXTCLOUD_CLI_INSTALL_ARCH="arm64" \
  sh "${repo_root}/install.sh" >"${tmp}/unsupported.log" 2>&1; then
  printf 'expected unsupported platform to fail\n' >&2
  exit 1
fi

grep 'unsupported platform: linux/arm64' "${tmp}/unsupported.log" >/dev/null

printf 'install script tests passed\n'
