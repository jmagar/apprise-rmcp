#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { binaryPath, downloadUrl, installRoot, releaseVersion, targetFor } = require("../lib/platform");

const DEFAULT_TIMEOUT_MS = 120_000;
const DEFAULT_CONNECT_TIMEOUT_MS = 10_000;
const DEFAULT_MAX_REDIRECTS = 5;

function log(message) {
  process.stderr.write(`apprise-rmcp: ${message}\n`);
}

function download(url, destination, options = {}) {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const connectTimeoutMs = options.connectTimeoutMs ?? DEFAULT_CONNECT_TIMEOUT_MS;
  const maxRedirects = options.maxRedirects ?? DEFAULT_MAX_REDIRECTS;
  const startedAt = options.startedAt ?? Date.now();
  const redirects = options.redirects ?? 0;

  return new Promise((resolve, reject) => {
    if (Date.now() - startedAt >= timeoutMs) {
      reject(new Error(`download timed out after ${timeoutMs}ms`));
      return;
    }
    const parsed = new URL(url);
    if (!["https:", "http:"].includes(parsed.protocol)) {
      reject(new Error(`unsupported download protocol ${parsed.protocol}`));
      return;
    }
    const client = parsed.protocol === "http:" ? http : https;
    let settled = false;
    const finish = (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(totalTimer);
      if (error) {
        fs.rmSync(destination, { force: true });
        reject(error);
      } else {
        resolve();
      }
    };
    const remaining = Math.max(1, timeoutMs - (Date.now() - startedAt));
    const totalTimer = setTimeout(() => request.destroy(new Error(`download timed out after ${timeoutMs}ms`)), remaining);
    const request = client.get(parsed, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
        response.resume();
        if (redirects >= maxRedirects) {
          finish(new Error(`download exceeded ${maxRedirects} redirects`));
          return;
        }
        if (!response.headers.location) {
          finish(new Error("download redirect omitted Location header"));
          return;
        }
        const next = new URL(response.headers.location, parsed).toString();
        settled = true;
        clearTimeout(totalTimer);
        download(next, destination, { timeoutMs, connectTimeoutMs, maxRedirects, startedAt, redirects: redirects + 1 }).then(resolve, reject);
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        finish(new Error(`download failed (${response.statusCode}) from ${url}`));
        return;
      }
      const file = fs.createWriteStream(destination, { flags: "wx", mode: 0o600 });
      response.pipe(file);
      file.on("finish", () => file.close((error) => finish(error)));
      file.on("error", finish);
      response.on("error", finish);
    });
    request.setTimeout(connectTimeoutMs, () => request.destroy(new Error(`download connection timed out after ${connectTimeoutMs}ms`)));
    request.on("error", finish);
  });
}

function sha256(filename) {
  return crypto.createHash("sha256").update(fs.readFileSync(filename)).digest("hex");
}

function verifyChecksum(archive, checksumFile) {
  const expected = fs.readFileSync(checksumFile, "utf8").trim().split(/\s+/u)[0];
  if (!/^[a-fA-F0-9]{64}$/u.test(expected)) throw new Error("malformed SHA256 file");
  const actual = sha256(archive);
  if (!crypto.timingSafeEqual(Buffer.from(actual, "hex"), Buffer.from(expected, "hex"))) {
    throw new Error(`SHA256 mismatch: expected ${expected}, got ${actual}`);
  }
}

function extractBinary(archive, destination, expectedBinary) {
  const listing = spawnSync("tar", ["-tzf", archive], { encoding: "utf8" });
  if (listing.status !== 0) throw new Error((listing.stderr || "tar listing failed").trim());
  const entries = listing.stdout.trim().split("\n").filter(Boolean);
  if (entries.length !== 1 || entries[0].replace(/^\.\//u, "") !== expectedBinary) {
    throw new Error(`archive must contain exactly ${expectedBinary}`);
  }
  fs.mkdirSync(destination, { recursive: true, mode: 0o700 });
  const result = spawnSync("tar", ["-xzf", archive, "-C", destination], { encoding: "utf8" });
  if (result.status !== 0) throw new Error((result.stderr || result.stdout || "tar extraction failed").trim());
  const candidate = path.join(destination, expectedBinary);
  const stat = fs.lstatSync(candidate);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error("archive binary is not a regular file");
  return candidate;
}

function atomicInstall(source, destination) {
  fs.mkdirSync(path.dirname(destination), { recursive: true, mode: 0o755 });
  const temporary = `${destination}.tmp-${process.pid}-${crypto.randomBytes(6).toString("hex")}`;
  try {
    fs.copyFileSync(source, temporary, fs.constants.COPYFILE_EXCL);
    fs.chmodSync(temporary, 0o755);
    fs.renameSync(temporary, destination);
  } finally {
    fs.rmSync(temporary, { force: true });
  }
}

async function main() {
  if (process.env.APPRISE_RMCP_SKIP_DOWNLOAD === "1") {
    log("skipping binary download because APPRISE_RMCP_SKIP_DOWNLOAD=1");
    return;
  }
  const target = targetFor();
  const destination = binaryPath();
  if (fs.existsSync(destination)) {
    log(`${path.basename(destination)} already installed for ${releaseVersion()}`);
    return;
  }
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "apprise-rmcp-install-"));
  const archive = path.join(tempDir, target.asset);
  const checksum = `${archive}.sha256`;
  try {
    const url = downloadUrl(target);
    log(`downloading ${url}`);
    const options = {
      timeoutMs: Number(process.env.APPRISE_RMCP_DOWNLOAD_TIMEOUT_MS || DEFAULT_TIMEOUT_MS),
      connectTimeoutMs: Number(process.env.APPRISE_RMCP_CONNECT_TIMEOUT_MS || DEFAULT_CONNECT_TIMEOUT_MS),
      maxRedirects: Number(process.env.APPRISE_RMCP_MAX_REDIRECTS || DEFAULT_MAX_REDIRECTS),
    };
    await download(url, archive, options);
    await download(`${url}.sha256`, checksum, options);
    verifyChecksum(archive, checksum);
    const staged = extractBinary(archive, path.join(tempDir, "staged"), target.binary);
    atomicInstall(staged, destination);
    log(`installed ${destination}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

module.exports = { atomicInstall, download, extractBinary, main, sha256, verifyChecksum };

if (require.main === module) {
  main().catch((error) => {
    log(error.message);
    process.exitCode = 1;
  });
}
