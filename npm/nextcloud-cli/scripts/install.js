#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const tar = require("tar");
const { binaryName, platformTriple } = require("../lib/platform");

const packageJson = require("../package.json");

async function main() {
  if (process.env.NEXTCLOUD_CLI_SKIP_DOWNLOAD === "1") {
    console.log("nextcloud-cli: skipping binary download");
    return;
  }
  if (process.env.NEXTCLOUD_CLI_BINARY) {
    console.log("nextcloud-cli: using NEXTCLOUD_CLI_BINARY");
    return;
  }

  const triple = platformTriple();
  const version = packageJson.version;
  const repo = process.env.NEXTCLOUD_CLI_RELEASE_REPO || "NicholaiVogel/Nextcloud-CLI";
  const asset = `nextcloud-cli-${version}-${triple}.tar.gz`;
  const releaseBase =
    process.env.NEXTCLOUD_CLI_RELEASE_BASE_URL ||
    `https://github.com/${repo}/releases/download/v${version}`;
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "nextcloud-cli-"));
  const archivePath = path.join(tmp, asset);
  const checksumPath = `${archivePath}.sha256`;
  const vendorDir = path.resolve(__dirname, "..", "vendor", triple);

  try {
    await download(`${releaseBase}/${asset}.sha256`, checksumPath);
    await download(`${releaseBase}/${asset}`, archivePath);
    verifyChecksum(archivePath, fs.readFileSync(checksumPath, "utf8"));

    fs.rmSync(vendorDir, { recursive: true, force: true });
    fs.mkdirSync(vendorDir, { recursive: true });
    await tar.x({ file: archivePath, cwd: vendorDir, strip: 1 });

    for (const command of ["nextcloud-cli", "nxc"]) {
      const binary = path.join(vendorDir, binaryName(command));
      if (!fs.existsSync(binary)) {
        throw new Error(`release archive did not contain ${binaryName(command)}`);
      }
      if (process.platform !== "win32") {
        fs.chmodSync(binary, 0o755);
      }
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (
        response.statusCode &&
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location
      ) {
        response.resume();
        download(response.headers.location, destination).then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`download failed: HTTP ${response.statusCode}`));
        return;
      }

      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    });
    request.on("error", reject);
  });
}

function verifyChecksum(file, checksumText) {
  const expected = checksumText.trim().split(/\s+/)[0]?.toLowerCase();
  if (!expected || !/^[a-f0-9]{64}$/.test(expected)) {
    throw new Error("invalid sha256 checksum file");
  }
  const actual = crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
  if (actual !== expected) {
    throw new Error(`sha256 mismatch for ${path.basename(file)}`);
  }
}

function sanitizeError(error) {
  return String(error?.message || error || "unknown error").replace(
    /https?:\/\/[^\s)]+/g,
    "<redacted-url>",
  );
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`nextcloud-cli install failed: ${sanitizeError(error)}`);
    console.error("Set NEXTCLOUD_CLI_SKIP_DOWNLOAD=1 to skip download during local development.");
    process.exit(1);
  });
}

module.exports = {
  main,
  sanitizeError,
  verifyChecksum,
};
