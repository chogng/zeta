import { cp, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const source = resolve("desktop/src/renderer/index.html");
const target = resolve("desktop/dist/src/renderer/index.html");
await mkdir(dirname(target), { recursive: true });
await cp(source, target);
