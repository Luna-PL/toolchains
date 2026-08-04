const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const [packageRoot, releaseTag] = process.argv.slice(2);
if (!packageRoot || !/^v\d+\.\d+\.\d+$/.test(releaseTag ?? "")) {
  throw new Error("usage: verify-installed-tools.js <package-root> <vX.Y.Z>");
}

const version = releaseTag.slice(1);
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const binary = (name) => join(packageRoot, "bin", `${name}${executableSuffix}`);

function run(name, args, input = undefined) {
  const result = spawnSync(binary(name), args, {
    encoding: "utf8",
    input,
    windowsHide: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `${name} exited with ${result.status}: ${result.stderr || result.stdout}`,
    );
  }
  return result;
}

const tools = run("luna-tools", []);
if (tools.stdout.trim() !== `LunaToolchain ${version}`) {
  throw new Error(`unexpected luna-tools version output: ${tools.stdout}`);
}

const formatter = run("luna-fmt", ["--help"]);
if (!formatter.stderr.includes("Usage:")) {
  throw new Error("luna-fmt did not print its usage text");
}

const requests = [
  { jsonrpc: "2.0", id: 1, method: "initialize", params: {} },
  { jsonrpc: "2.0", id: 2, method: "shutdown", params: null },
  { jsonrpc: "2.0", method: "exit", params: null },
];
const input = requests
  .map((request) => {
    const body = JSON.stringify(request);
    return `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`;
  })
  .join("");
const lsp = run("luna-lsp", [], input);
if (!lsp.stdout.includes('"id":1') || !lsp.stdout.includes('"id":2')) {
  throw new Error("luna-lsp did not complete initialize and shutdown");
}

const compatibility = JSON.parse(
  readFileSync(join(packageRoot, "compatibility", "luna.json"), "utf8"),
);
if (compatibility.toolchain_version !== version) {
  throw new Error("packaged compatibility manifest has the wrong toolchain version");
}

console.log(`installed LunaToolchain ${version} tools verified`);
