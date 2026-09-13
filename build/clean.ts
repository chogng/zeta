import { lstat, readdir, rm, unlink } from "node:fs/promises";
import { join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "..");
const outputRoots = [
  join(repositoryRoot, ".build"),
  join(repositoryRoot, "build", "release", "__pycache__"),
  join(repositoryRoot, "build", "release", "ash_package", "__pycache__"),
  join(repositoryRoot, "ash-ts", ".tmp"),
  join(repositoryRoot, "ash-ts", "build"),
  join(repositoryRoot, "ash-ts", "dist"),
  join(repositoryRoot, "ash-ts", "output"),
  join(repositoryRoot, "output"),
  join(repositoryRoot, "target"),
  join(repositoryRoot, "ash-rs", "target"),
];

for (const outputRoot of outputRoots) {
  await removeOutputRoot(outputRoot);
}
console.log("Removed local build outputs.");

async function removeOutputRoot(root: string): Promise<void> {
  let metadata;
  try {
    metadata = await lstat(root);
  } catch (error) {
    if (error?.code === "ENOENT") return;
    throw error;
  }
  if (metadata.isSymbolicLink()) {
    await unlink(root);
    return;
  }
  for (const entry of await readdir(root, { withFileTypes: true })) {
    if (entry.isSymbolicLink()) await unlink(join(root, entry.name));
  }
  await rm(root, { force: true, recursive: true });
}
