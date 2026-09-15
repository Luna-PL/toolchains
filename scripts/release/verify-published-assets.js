const { createHash } = require("node:crypto");
const { readFileSync } = require("node:fs");
const { basename, join } = require("node:path");

const [directory, releaseTag, ...downloadedNames] = process.argv.slice(2);
if (!directory || !/^v\d+\.\d+\.\d+$/.test(releaseTag ?? "")) {
  throw new Error("usage: verify-published-assets.js <directory> <vX.Y.Z> <asset>...");
}
if (downloadedNames.length === 0) {
  throw new Error("at least one downloaded asset must be verified");
}

const version = releaseTag.slice(1);
const expectedNames = [
  "LUNA-SOURCE-COMMIT",
  `luna-language-${releaseTag}-darwin-arm64.vsix`,
  `luna-language-${releaseTag}-linux-x64.vsix`,
  `luna-language-${releaseTag}-win32-x64.vsix`,
  `luna-toolchain-${version}-linux-x86_64.tar.gz`,
  `luna-toolchain-${version}-macos-arm64.tar.gz`,
  `luna-toolchain-${version}-windows-x86_64.zip`,
];
const checksumLines = readFileSync(join(directory, "SHA256SUMS"), "utf8")
  .trim()
  .split(/\r?\n/);
const checksums = new Map();
for (const line of checksumLines) {
  const match = line.match(/^([0-9a-f]{64})  (.+)$/);
  if (!match || basename(match[2]) !== match[2]) {
    throw new Error(`invalid SHA256SUMS line: ${line}`);
  }
  if (checksums.has(match[2])) {
    throw new Error(`duplicate checksum entry: ${match[2]}`);
  }
  checksums.set(match[2], match[1]);
}

if (
  checksums.size !== expectedNames.length ||
  expectedNames.some((name) => !checksums.has(name))
) {
  throw new Error("SHA256SUMS does not describe the complete release asset set");
}

for (const name of downloadedNames) {
  if (basename(name) !== name || !checksums.has(name)) {
    throw new Error(`unexpected asset name: ${name}`);
  }
  const digest = createHash("sha256")
    .update(readFileSync(join(directory, name)))
    .digest("hex");
  if (digest !== checksums.get(name)) {
    throw new Error(`checksum mismatch for ${name}`);
  }
  console.log(`${name}: ${digest}`);
}

const compatibility = JSON.parse(
  readFileSync(join(__dirname, "..", "..", "compatibility", "luna.json"), "utf8"),
);
const publishedLunaCommit = readFileSync(
  join(directory, "LUNA-SOURCE-COMMIT"),
  "utf8",
).trim();
if (publishedLunaCommit !== compatibility.luna?.source_commit) {
  throw new Error(
    `published Luna source commit ${publishedLunaCommit} does not match compatibility metadata`,
  );
}

console.log(`complete ${releaseTag} checksum manifest verified`);
