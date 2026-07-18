#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");
const { binaryPath } = require("../lib/platform");

function fail(message) {
  process.stderr.write(`apprise-rmcp: ${message}\n`);
  process.exit(1);
}

const binary = binaryPath();
if (!fs.existsSync(binary)) {
  const installer = path.resolve(__dirname, "..", "scripts", "install.js");
  const install = spawnSync(process.execPath, [installer], { stdio: "inherit" });
  if (install.status !== 0) fail("binary is not installed; postinstall may have failed");
}

const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
const forwarded = ["SIGINT", "SIGTERM", "SIGHUP"];
const signalHandlers = new Map();
for (const signal of forwarded) {
  const handler = () => {
    if (!child.killed) child.kill(signal);
  };
  signalHandlers.set(signal, handler);
  process.on(signal, handler);
}
child.on("error", (error) => fail(error.message));
child.on("exit", (code, signal) => {
  for (const [forwardedSignal, handler] of signalHandlers) {
    process.removeListener(forwardedSignal, handler);
  }
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
