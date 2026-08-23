import { join, resolve } from "node:path";

/** Resolves Cargo's shared target directory using the same cwd semantics as the build. */
export function cargoTargetDirectory(workspaceRoot: string, environment: Readonly<Record<string, string | undefined>> = process.env): string {
  const configured = environment.CARGO_TARGET_DIR?.trim();
  return configured ? resolve(workspaceRoot, configured) : join(workspaceRoot, ".build", "cargo");
}

/** Parses one line emitted by Cargo's JSON message format. */
export function parseCargoMessage(line: string): unknown {
  try {
    return JSON.parse(line);
  } catch {
    return undefined;
  }
}

/** Returns the executable path from a matching Cargo binary artifact message. */
export function cargoArtifactExecutable(message: unknown, targetName: string): string | undefined {
  const record = objectRecord(message);
  const target = objectRecord(record?.target);
  return record?.reason === "compiler-artifact"
    && target?.name === targetName
    && Array.isArray(target.kind)
    && target.kind.includes("bin")
    && typeof record.executable === "string"
    ? record.executable
    : undefined;
}

/** Returns a rendered compiler diagnostic carried by Cargo's JSON stream. */
export function cargoRenderedDiagnostic(message: unknown): string | undefined {
  const record = objectRecord(message);
  const diagnostic = objectRecord(record?.message);
  return record?.reason === "compiler-message" && typeof diagnostic?.rendered === "string"
    ? diagnostic.rendered
    : undefined;
}

function objectRecord(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === "object" ? value as Record<string, unknown> : undefined;
}
