import { copyFile, mkdir, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const repositoryDirectory = resolve(import.meta.dirname, "../../..");
const desktopDirectory = resolve(repositoryDirectory, "desktop");
const source = resolve(
  repositoryDirectory,
  "zeta-rs/app-server-protocol/schema/types.ts",
);
const destination = resolve(desktopDirectory, "generated/app-server/types.ts");
const staleJavaScriptDestination = resolve(desktopDirectory, "generated/app-server/types.js");

await mkdir(dirname(destination), { recursive: true });
await copyFile(source, destination);
await rm(staleJavaScriptDestination, { force: true });
