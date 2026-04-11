"use strict";

function platformTriple(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (platform === "linux" && arch === "arm64") {
    return "aarch64-unknown-linux-gnu";
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
  if (platform === "win32" && arch === "arm64") {
    return "aarch64-pc-windows-msvc";
  }

  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

function binaryName(command = "nextcloud-cli", platform = process.platform) {
  const base = command === "nxc" ? "nxc" : "nextcloud-cli";
  return platform === "win32" ? `${base}.exe` : base;
}

module.exports = {
  binaryName,
  platformTriple,
};
