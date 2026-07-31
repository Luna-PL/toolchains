const { spawnSync } = require("node:child_process");
const { existsSync, mkdirSync } = require("node:fs");
const { join } = require("node:path");

const sourceRoot = process.env.LUNA_SOURCE_DIR;
if (!sourceRoot) {
  console.error("LUNA_SOURCE_DIR must point at a Luna compiler checkout");
  process.exit(2);
}

const root = join(__dirname, "..");
const build = join(root, "build");
const extension =
  process.platform === "win32"
    ? "dll"
    : process.platform === "darwin"
      ? "dylib"
      : "so";
const library = join(build, `luna.${extension}`);
const cli = join(
  root,
  "node_modules",
  "tree-sitter-cli",
  "cli.js",
);
const environment = {
  ...process.env,
  XDG_CACHE_HOME: join(build, "cache"),
};
mkdirSync(build, { recursive: true });
if (!existsSync(library)) {
  const buildResult = spawnSync(process.execPath, [cli, "build", "-o", library, root], {
    cwd: root,
    env: environment,
    encoding: "utf8",
    stdio: "inherit",
  });
  if (buildResult.status !== 0) process.exit(buildResult.status ?? 1);
}

const fixtures = [
  "examples/minimal.luna",
  "examples/generic.luna",
  "examples/fragments.luna",
  "examples/operators.luna",
  "examples/ffi.luna",
  "examples/full_showcase/foundation/src/dispatch.luna",
  "examples/full_showcase/foundation/src/model.luna",
  "examples/full_showcase/app/src/main.luna",
  "tests/fixtures/result_match.luna",
  "tests/fixtures/iterator_pipeline.luna",
  "tests/fixtures/dynamic_fragments.luna",
  "tests/fixtures/comparison_operators.luna",
  "tests/fixtures/type_relations.luna",
];

for (const fixture of fixtures) {
  const path = join(sourceRoot, fixture);
  if (!existsSync(path)) {
    console.error(`missing compiler fixture: ${path}`);
    process.exit(2);
  }
  const result = spawnSync(
    process.execPath,
    [cli, "parse", "--quiet", "-l", library, "--lang-name", "luna", path],
    { encoding: "utf8", env: environment },
  );
  if (result.status !== 0) {
    console.error(`grammar rejected compiler fixture ${fixture}`);
    process.stderr.write(result.stderr);
    process.stdout.write(result.stdout);
    process.exit(1);
  }
}

console.log(`parsed ${fixtures.length} compiler fixtures`);
