"use strict";

const path = require("node:path");

function packageVersion() {
  return require("../package.json").version;
}

function targetFor(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") {
    return {
      asset: "rapprise-x86_64.tar.gz",
      binary: "rapprise",
    };
  }

  if (platform === "win32" && arch === "x64") {
    return {
      asset: "rapprise-windows-x86_64.tar.gz",
      binary: "rapprise.exe",
    };
  }

  throw new Error(`Unsupported platform ${platform}/${arch}. Supported targets: linux/x64, win32/x64.`);
}

function releaseVersion(env = process.env) {
  const raw = env.APPRISE_RMCP_BINARY_VERSION || env.APPRISE_RMCP_VERSION || packageVersion();
  return raw.startsWith("v") ? raw : `v${raw}`;
}

function releaseBaseUrl(env = process.env) {
  const repo = env.APPRISE_RMCP_REPO || "jmagar/apprise-rmcp";
  return env.APPRISE_RMCP_RELEASE_BASE_URL || `https://github.com/${repo}/releases/download`;
}

function downloadUrl(target, env = process.env) {
  return `${releaseBaseUrl(env)}/${releaseVersion(env)}/${target.asset}`;
}

function installRoot() {
  return path.resolve(__dirname, "..", "vendor");
}

function binaryPath(platform = process.platform, arch = process.arch) {
  const target = targetFor(platform, arch);
  return path.join(installRoot(), target.binary);
}

module.exports = {
  binaryPath,
  downloadUrl,
  releaseBaseUrl,
  installRoot,
  packageVersion,
  releaseVersion,
  targetFor,
};
