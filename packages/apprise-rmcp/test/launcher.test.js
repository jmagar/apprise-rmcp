"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

test("launcher exits with the native child's terminating signal", { skip: process.platform === "win32" }, (t) => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "rapprise-launcher-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));

  const packageRoot = path.join(directory, "apprise-rmcp");
  fs.cpSync(path.resolve(__dirname, ".."), packageRoot, { recursive: true });
  const binary = path.join(packageRoot, "vendor", "rapprise");
  fs.mkdirSync(path.dirname(binary), { recursive: true });
  fs.writeFileSync(binary, "#!/bin/sh\nkill -TERM $$\n", { mode: 0o755 });

  const result = spawnSync(process.execPath, [path.join(packageRoot, "bin", "rapprise.js")], {
    timeout: 1000,
  });

  assert.equal(result.error, undefined, `launcher timed out: ${result.error}`);
  assert.equal(result.signal, "SIGTERM");
});
