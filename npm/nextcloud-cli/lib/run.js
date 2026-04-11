"use strict";

const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { binaryName, platformTriple } = require("./platform");

function packageRoot() {
  return path.resolve(__dirname, "..");
}

function resolveBinary(command = "nextcloud-cli") {
  if (process.env.NEXTCLOUD_CLI_BINARY) {
    return process.env.NEXTCLOUD_CLI_BINARY;
  }

  const triple = platformTriple();
  return path.join(packageRoot(), "vendor", triple, binaryName(command));
}

function runInstaller() {
  const installer = path.join(packageRoot(), "scripts", "install.js");
  return spawnSync(process.execPath, [installer], { stdio: "inherit" });
}

function ensureBinary(command = "nextcloud-cli") {
  let binary = resolveBinary(command);
  if (fs.existsSync(binary)) {
    return binary;
  }

  if (!process.env.NEXTCLOUD_CLI_BINARY) {
    const result = runInstaller();
    if (result.error) {
      console.error(`nextcloud-cli install recovery failed: ${result.error.message}`);
    }
    binary = resolveBinary(command);
  }

  return binary;
}

function run(command = "nextcloud-cli") {
  const binary = ensureBinary(command);
  if (!fs.existsSync(binary)) {
    console.error(
      [
        `nextcloud-cli binary was not found at ${binary}`,
        "Reinstall the package, or set NEXTCLOUD_CLI_BINARY to a local binary.",
      ].join("\n"),
    );
    process.exit(127);
  }

  const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
  child.on("error", (error) => {
    console.error(`failed to execute ${binary}: ${error.message}`);
    process.exit(127);
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
}

module.exports = {
  ensureBinary,
  packageRoot,
  resolveBinary,
  run,
  runInstaller,
};
