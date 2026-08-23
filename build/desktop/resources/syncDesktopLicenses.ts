import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(import.meta.dirname, "../../..");

const licenseCopies = [
  {
    source: resolve(repositoryDirectory, "zeta-rs/file-icons/seti/LICENSE.txt"),
    destination: resolve(repositoryDirectory, "zeta-ts/licenses/Seti-UI.txt"),
  },
  ...["Typst.txt", "Typst-NOTICE.txt", "Typst-Assets-NOTICE.txt"].map((fileName) => ({
    source: resolve(repositoryDirectory, "zeta-rs/utils/typst/licenses", fileName),
    destination: resolve(repositoryDirectory, "zeta-ts/licenses", fileName),
  })),
];

export async function syncDesktopLicenseCopies(): Promise<{ readonly changed: readonly string[]; readonly count: number }> {
  const changed: string[] = [];
  for (const licenseCopy of licenseCopies) {
    const source = await readFile(licenseCopy.source);
    const destination = await readFileIfPresent(licenseCopy.destination);
    if (destination?.equals(source)) {
      continue;
    }
    await mkdir(dirname(licenseCopy.destination), { recursive: true });
    await writeFile(licenseCopy.destination, source);
    changed.push(repositoryRelativePath(licenseCopy.destination));
  }
  return { changed, count: licenseCopies.length };
}

export async function checkDesktopLicenseCopies(): Promise<{ readonly count: number }> {
  const mismatches: string[] = [];
  for (const licenseCopy of licenseCopies) {
    const source = await readFile(licenseCopy.source);
    const destination = await readFileIfPresent(licenseCopy.destination);
    if (!destination?.equals(source)) {
      mismatches.push(repositoryRelativePath(licenseCopy.destination));
    }
  }
  if (mismatches.length > 0) {
    throw new Error(`Desktop license copies require synchronization: ${mismatches.join(", ")}. Run 'corepack pnpm sync:desktop-licenses'.`);
  }
  return { count: licenseCopies.length };
}

async function readFileIfPresent(path: string): Promise<Buffer | undefined> {
  try {
    return await readFile(path);
  } catch (error: unknown) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") {
      return undefined;
    }
    throw error;
  }
}

function repositoryRelativePath(path: string): string {
  return relative(repositoryDirectory, path).replaceAll("\\", "/");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const argumentsList = process.argv.slice(2);
  if (argumentsList.length > 1 || (argumentsList.length === 1 && argumentsList[0] !== "--check")) {
    throw new Error(`Expected --check or no arguments; received ${argumentsList.join(" ")}`);
  }
  if (argumentsList[0] === "--check") {
    const report = await checkDesktopLicenseCopies();
    console.log(`Validated ${report.count} Desktop license copies.`);
  } else {
    const report = await syncDesktopLicenseCopies();
    const status = report.changed.length > 0 ? `Synchronized ${report.changed.join(", ")}` : "Desktop license copies are already synchronized";
    console.log(`${status}.`);
  }
}
