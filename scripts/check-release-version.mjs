import { readFile } from "node:fs/promises";

const tag = process.argv[2];

if (!tag?.startsWith("v")) {
  console.error("Release tags must use vX.Y.Z format");
  process.exit(1);
}

const expected = tag.slice(1);

const files = [
  "apps/desktop/package.json",
  "apps/desktop/src-tauri/tauri.conf.json",
];

for (const file of files) {
  const content = await readFile(file, "utf8");
  const json = JSON.parse(content);

  if (json.version !== expected) {
    console.error(`${file}: version ${json.version} does not match ${expected}`);
    process.exit(1);
  }
}

const cargo = await readFile("Cargo.toml", "utf8");
if (!cargo.includes("repository = \"https://github.com/fightingPig/my-agent-assets\"")) {
  console.error("Cargo repository metadata does not point to the release repository");
  process.exit(1);
}

console.log(`Release version ${expected} validated`);
