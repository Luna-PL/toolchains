const { mkdirSync } = require("node:fs");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

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

for (const arguments of [
  ["build", "-o", library, root],
  ["test", "-l", library, "--lang-name", "luna"],
]) {
  const result = spawnSync(process.execPath, [cli, ...arguments], {
    cwd: root,
    env: environment,
    encoding: "utf8",
    stdio: "inherit",
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
