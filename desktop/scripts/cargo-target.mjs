import { join, resolve } from "node:path";

/** Resolves Cargo's shared target directory using the same cwd semantics as the build. */
export function cargoTargetDirectory(workspaceRoot, environment = process.env) {
  const configured = environment.CARGO_TARGET_DIR?.trim();
  return configured ? resolve(workspaceRoot, configured) : join(workspaceRoot, "target");
}

/** Parses one line emitted by Cargo's JSON message format. */
export function parseCargoMessage(line) {
  try {
    return JSON.parse(line);
  } catch {
    return undefined;
  }
}

/** Returns the executable path from a matching Cargo binary artifact message. */
export function cargoArtifactExecutable(message, targetName) {
  return message?.reason === "compiler-artifact"
    && message.target?.name === targetName
    && Array.isArray(message.target.kind)
    && message.target.kind.includes("bin")
    && typeof message.executable === "string"
    ? message.executable
    : undefined;
}

/** Returns a rendered compiler diagnostic carried by Cargo's JSON stream. */
export function cargoRenderedDiagnostic(message) {
  return message?.reason === "compiler-message" && typeof message.message?.rendered === "string"
    ? message.message.rendered
    : undefined;
}
