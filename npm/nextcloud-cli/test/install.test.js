"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { sanitizeError, verifyChecksum } = require("../scripts/install");

test("sanitizes URLs from installer errors", () => {
  const error = new Error("failed https://example.com/releases/file.tar.gz?token=secret");
  assert.equal(sanitizeError(error), "failed <redacted-url>");
});

test("rejects malformed checksum files", () => {
  assert.throws(() => verifyChecksum(__filename, "not-a-sha"), /invalid sha256/);
});
