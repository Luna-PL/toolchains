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
const luna = compatibility.luna;

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

if (!/^v\d+\.\d+\.\d+$/.test(luna?.release_tag ?? "")) {
  throw new Error(`invalid Luna release tag: ${luna?.release_tag ?? "<missing>"}`);
}
if (luna.release_tag !== `v${luna.language_version}`) {
  throw new Error(
    `Luna release tag ${luna.release_tag} does not match language version ${luna.language_version}`,
  );
}
if (!/^[0-9a-f]{40}$/.test(luna.source_commit ?? "")) {
  throw new Error("Luna source_commit must be a full lowercase 40-hex SHA");
}

console.log(
  `release versions agree on ${expected}; Luna baseline is ${luna.release_tag} (${luna.source_commit})`,
);
