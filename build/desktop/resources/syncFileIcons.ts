import { copyFile, mkdir, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const repositoryDirectory = resolve(import.meta.dirname, "../../..");
const desktopDirectory = resolve(repositoryDirectory, "desktop");
const crateDirectory = resolve(repositoryDirectory, "zeta-rs/file-icons");

await rm(resolve(desktopDirectory, "generated/file-icons/types.ts"), {
  force: true,
});

const artifacts = [
  [
    resolve(crateDirectory, "seti/manifest.json"),
    resolve(desktopDirectory, "generated/file-icons/seti/manifest.json"),
  ],
  [
    resolve(crateDirectory, "seti/seti.woff"),
    resolve(desktopDirectory, "generated/file-icons/seti/seti.woff"),
  ],
  [
    resolve(crateDirectory, "seti/LICENSE.txt"),
    resolve(desktopDirectory, "licenses/Seti-UI.txt"),
  ],
];

for (const [source, destination] of artifacts) {
  await mkdir(dirname(destination), { recursive: true });
  await copyFile(source, destination);
}
