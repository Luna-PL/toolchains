const { chmodSync, copyFileSync, existsSync, mkdirSync } = require("node:fs");
const { join } = require("node:path");

const extensionRoot = join(__dirname, "..");
const repositoryRoot = join(extensionRoot, "..", "..");
const releaseRoot = join(repositoryRoot, "target", "release");
const binRoot = join(extensionRoot, "bin");
const suffix = process.platform === "win32" ? ".exe" : "";

mkdirSync(binRoot, { recursive: true });
for (const name of ["luna-lsp", "luna-fmt"]) {
  const fileName = `${name}${suffix}`;
  const source = join(releaseRoot, fileName);
  if (!existsSync(source)) {
    throw new Error(`missing release binary: ${source}`);
  }
  const destination = join(binRoot, fileName);
  copyFileSync(source, destination);
  if (process.platform !== "win32") chmodSync(destination, 0o755);
}

copyFileSync(
  join(repositoryRoot, "compatibility", "luna.json"),
  join(extensionRoot, "compatibility.json"),
);
