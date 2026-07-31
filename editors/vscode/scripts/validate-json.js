const { readFileSync } = require("node:fs");
const { join } = require("node:path");

const root = join(__dirname, "..");

for (const relative of [
  "language-configuration.json",
  "package.json",
  "syntaxes/luna.tmLanguage.json",
]) {
  const path = join(root, relative);
  try {
    JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`${relative}: ${error.message}`, { cause: error });
  }
}
