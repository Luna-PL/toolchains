const { readFileSync } = require("node:fs");
const { join } = require("node:path");

const root = join(__dirname, "..", "..");
const releaseTag = process.env.RELEASE_TAG;
if (!releaseTag || !/^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(releaseTag)) {
  throw new Error(`invalid RELEASE_TAG: ${releaseTag ?? "<unset>"}`);
}

const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
const cargoVersion = cargo.match(
  /\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/,
)?.[1];
const extension = JSON.parse(
  readFileSync(join(root, "editors", "vscode", "package.json"), "utf8"),
);
const grammar = JSON.parse(
  readFileSync(join(root, "grammars", "tree-sitter-luna", "package.json"), "utf8"),
);
const generatedParser = readFileSync(
  join(root, "grammars", "tree-sitter-luna", "src", "parser.c"),
  "utf8",
);
const generatedVersionParts = ["major", "minor", "patch"].map((part) =>
  generatedParser.match(new RegExp(`\\.${part}_version = (\\d+),`))?.[1],
);
const generatedVersion = generatedVersionParts.every(Boolean)
  ? generatedVersionParts.join(".")
  : undefined;
const compatibility = JSON.parse(
  readFileSync(join(root, "compatibility", "luna.json"), "utf8"),
);
const expected = releaseTag.slice(1);

for (const [owner, version] of [
  ["Cargo workspace", cargoVersion],
  ["VS Code extension", extension.version],
  ["Tree-sitter grammar", grammar.version],
  ["Tree-sitter generated parser", generatedVersion],
  ["compatibility manifest", compatibility.toolchain_version],
]) {
  if (version !== expected) {
    throw new Error(`${owner} version ${version ?? "<missing>"} does not match ${releaseTag}`);
  }
}

console.log(`release versions agree on ${expected}`);
