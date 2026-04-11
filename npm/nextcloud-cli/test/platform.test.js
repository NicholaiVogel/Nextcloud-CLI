"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { binaryName, platformTriple } = require("../lib/platform");

test("maps supported platforms to Rust host triples", () => {
  assert.equal(platformTriple("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.equal(platformTriple("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(platformTriple("win32", "x64"), "x86_64-pc-windows-msvc");
});

test("adds exe suffix for Windows binaries", () => {
  assert.equal(binaryName("nextcloud-cli", "linux"), "nextcloud-cli");
  assert.equal(binaryName("nxc", "win32"), "nxc.exe");
});

test("rejects unsupported platforms", () => {
  assert.throws(() => platformTriple("freebsd", "x64"), /Unsupported platform/);
});
