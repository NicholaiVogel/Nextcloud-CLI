"use strict";

function platformTriple(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (platform === "win32" && arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }
  throw new Error(
    `Unsupported platform: ${platform}/${arch}. Supported npm release platforms are linux/x64, darwin/x64, darwin/arm64, and win32/x64.`,
  );
}

function binaryName(command = "nextcloud-cli", platform = process.platform) {
  const base = command === "nxc" ? "nxc" : "nextcloud-cli";
  return platform === "win32" ? `${base}.exe` : base;
}

module.exports = {
  binaryName,
  platformTriple,
};
