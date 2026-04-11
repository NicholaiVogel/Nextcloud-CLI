"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { resolveBinary } = require("../lib/run");

test("NEXTCLOUD_CLI_BINARY overrides vendor binary lookup", () => {
  const previous = process.env.NEXTCLOUD_CLI_BINARY;
  process.env.NEXTCLOUD_CLI_BINARY = "/tmp/nextcloud-cli";
  try {
    assert.equal(resolveBinary("nxc"), "/tmp/nextcloud-cli");
  } finally {
    if (previous === undefined) {
      delete process.env.NEXTCLOUD_CLI_BINARY;
    } else {
      process.env.NEXTCLOUD_CLI_BINARY = previous;
    }
  }
});
