#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const zlib = require("zlib");

const REPO = "skeletor-js/medulla";
const PACKAGE_VERSION = require("./package.json").version;

// Map Node.js platform/arch to Rust target triples
function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  const targets = {
    darwin: {
      x64: "x86_64-apple-darwin",
      arm64: "aarch64-apple-darwin",
    },
    linux: {
      x64: "x86_64-unknown-linux-gnu",
      arm64: "aarch64-unknown-linux-gnu",
    },
    win32: {
      x64: "x86_64-pc-windows-msvc",
    },
  };

  const platformTargets = targets[platform];
  if (!platformTargets) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  const target = platformTargets[arch];
  if (!target) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  return target;
}

// Download file with redirect following
function download(url) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        // Follow redirect
        download(response.headers.location).then(resolve).catch(reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      const chunks = [];
      response.on("data", (chunk) => chunks.push(chunk));
      response.on("end", () => resolve(Buffer.concat(chunks)));
      response.on("error", reject);
    });

    request.on("error", reject);
  });
}

// Extract tar.gz
function extractTarGz(buffer, destDir) {
  const gunzipped = zlib.gunzipSync(buffer);

  // Simple tar extraction (handles single file)
  // tar format: 512-byte header followed by file content
  let offset = 0;
  while (offset < gunzipped.length) {
    const header = gunzipped.slice(offset, offset + 512);

    // Check for empty block (end of archive)
    if (header.every((b) => b === 0)) break;

    // Extract filename (first 100 bytes, null-terminated)
    const nameEnd = header.indexOf(0);
    const name = header.slice(0, Math.min(nameEnd, 100)).toString("utf8");

    // Extract file size (octal, bytes 124-135)
    const sizeStr = header.slice(124, 136).toString("utf8").trim();
    const size = parseInt(sizeStr, 8) || 0;

    offset += 512; // Move past header

    if (name && size > 0 && !name.endsWith("/")) {
      const content = gunzipped.slice(offset, offset + size);
      const destPath = path.join(destDir, path.basename(name));
      fs.writeFileSync(destPath, content, { mode: 0o755 });
    }

    // Move to next 512-byte boundary
    offset += Math.ceil(size / 512) * 512;
  }
}

async function main() {
  try {
    const target = getTarget();
    const isWindows = process.platform === "win32";
    const ext = isWindows ? "zip" : "tar.gz";
    const version = `v${PACKAGE_VERSION}`;

    const url = `https://github.com/${REPO}/releases/download/${version}/medulla-${version}-${target}.${ext}`;

    console.log(`Downloading medulla ${version} for ${target}...`);

    const buffer = await download(url);

    const binDir = path.join(__dirname, "bin");

    if (isWindows) {
      // For Windows, we'd need to extract zip
      // For now, just save the binary directly
      console.error("Windows zip extraction not yet implemented");
      process.exit(1);
    } else {
      extractTarGz(buffer, binDir);
    }

    console.log("medulla installed successfully!");
  } catch (error) {
    console.error("Failed to install medulla:", error.message);
    console.error("");
    console.error("You can install manually from:");
    console.error(`  https://github.com/${REPO}/releases`);
    process.exit(1);
  }
}

main();
