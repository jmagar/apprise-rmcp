"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { atomicInstall, download, verifyChecksum } = require("../scripts/install");

function listen(handler) {
  return new Promise((resolve) => {
    const server = http.createServer(handler);
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

test("download follows bounded redirects and removes partial failures", async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rapprise-download-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const server = await listen((request, response) => {
    if (request.url === "/start") {
      response.writeHead(302, { Location: "/payload" });
      response.end();
    } else {
      response.end("verified payload");
    }
  });
  t.after(() => server.close());
  const destination = path.join(dir, "payload");
  await download(`http://127.0.0.1:${server.address().port}/start`, destination, { timeoutMs: 1000, maxRedirects: 1 });
  assert.equal(fs.readFileSync(destination, "utf8"), "verified payload");
});

test("download rejects redirect loops and deadlines", async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rapprise-deadline-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const server = await listen((request, response) => {
    if (request.url === "/loop") {
      response.writeHead(302, { Location: "/loop" });
      response.end();
    }
  });
  t.after(() => server.close());
  await assert.rejects(
    download(`http://127.0.0.1:${server.address().port}/loop`, path.join(dir, "loop"), { timeoutMs: 1000, maxRedirects: 1 }),
    /exceeded 1 redirects/,
  );
  await assert.rejects(
    download(`http://127.0.0.1:${server.address().port}/stall`, path.join(dir, "stall"), { timeoutMs: 30, connectTimeoutMs: 1000 }),
    /timed out/,
  );
  assert.equal(fs.existsSync(path.join(dir, "stall")), false);
});

test("checksum verification rejects tampering", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rapprise-checksum-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const archive = path.join(dir, "archive");
  const checksum = `${archive}.sha256`;
  fs.writeFileSync(archive, "good");
  fs.writeFileSync(checksum, "770e607624d689265ca6c44884d0807d9b054d23c473c106c72be9de08b7376c  archive\n");
  verifyChecksum(archive, checksum);
  fs.writeFileSync(archive, "tampered");
  assert.throws(() => verifyChecksum(archive, checksum), /SHA256 mismatch/);
});

test("atomic install preserves the old binary until replacement is ready", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rapprise-atomic-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const source = path.join(dir, "source");
  const destination = path.join(dir, "bin", "rapprise");
  fs.mkdirSync(path.dirname(destination));
  fs.writeFileSync(destination, "old");
  fs.writeFileSync(source, "new");
  atomicInstall(source, destination);
  assert.equal(fs.readFileSync(destination, "utf8"), "new");
  assert.equal(fs.statSync(destination).mode & 0o777, 0o755);
});
