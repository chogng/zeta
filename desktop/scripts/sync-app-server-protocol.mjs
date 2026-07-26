import { copyFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = resolve(desktopDirectory, "..");
const source = resolve(
  repositoryDirectory,
  "zeta-rs/app-server-protocol/schema/types.ts",
);
const destination = resolve(desktopDirectory, "generated/app-server/types.ts");

await mkdir(dirname(destination), { recursive: true });
await copyFile(source, destination);
