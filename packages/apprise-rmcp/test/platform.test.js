"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  binaryVersion,
  downloadUrl,
  releaseBaseUrl,
  releaseVersion,
  targetFor,
} = require("../lib/platform");
const { binaryVersion: pinnedBinaryVersion } = require("../package.json");

test("maps supported platforms to release assets", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "rapprise-x86_64.tar.gz",
    binary: "rapprise",
  });
  assert.deepEqual(targetFor("win32", "x64"), {
    asset: "rapprise-windows-x86_64.tar.gz",
    binary: "rapprise.exe",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("darwin", "arm64"), /Unsupported platform/);
});

test("uses pinned binary version as the binary tag by default", () => {
  assert.equal(binaryVersion(), pinnedBinaryVersion);
  assert.equal(releaseVersion({}), `v${pinnedBinaryVersion}`);
});

test("allows release tag and repo overrides", () => {
  const env = { APPRISE_RMCP_BINARY_VERSION: "v9.9.9", APPRISE_RMCP_REPO: "example/apprise-rmcp" };
  assert.equal(releaseBaseUrl(env), "https://github.com/example/apprise-rmcp/releases/download");
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://github.com/example/apprise-rmcp/releases/download/v9.9.9/rapprise-x86_64.tar.gz");
});
