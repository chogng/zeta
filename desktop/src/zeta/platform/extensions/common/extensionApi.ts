export type ExtensionCatalogReload = "cached" | "refresh";
export type ExtensionSourceKind = "builtIn" | "user";
export type ExtensionDiagnosticCode = "sourceUnavailable" | "invalidManifest" | "duplicateExtension" | "pathEscapesRoot" | "resourceNotFound" | "resourceTooLarge";

export interface ExtensionDescriptor {
  readonly id: string;
  readonly name: string;
  readonly publisher: string;
  readonly version: string;
  readonly displayName: string;
  readonly sourceKind: ExtensionSourceKind;
  readonly manifestJson: string;
  readonly manifestSha256: string;
}

export interface ExtensionDiagnostic {
  readonly source: string;
  readonly subject: string | undefined;
  readonly code: ExtensionDiagnosticCode;
  readonly message: string;
}

export interface ExtensionCatalog {
  readonly generation: number;
  readonly extensions: readonly ExtensionDescriptor[];
  readonly diagnostics: readonly ExtensionDiagnostic[];
}

/** Renderer-facing capability for reading static, Rust-validated extension resources. */
export interface IExtensionApi {
  list(reload: ExtensionCatalogReload): Promise<ExtensionCatalog>;
  readResource(extensionId: string, path: string): Promise<Uint8Array>;
}

export function normalizeExtensionCatalog(value: unknown): ExtensionCatalog {
  const catalog = record(value, "extension catalog");
  return Object.freeze({
    generation: nonNegativeSafeInteger(catalog.generation, "extension catalog generation"),
    extensions: Object.freeze(array(catalog.extensions, "extensions").map(normalizeExtension)),
    diagnostics: Object.freeze(array(catalog.diagnostics, "diagnostics").map(normalizeDiagnostic)),
  });
}

function normalizeExtension(value: unknown): ExtensionDescriptor {
  const extension = record(value, "extension");
  return Object.freeze({
    id: boundedText(extension.id, "extension id", 160),
    name: boundedText(extension.name, "extension name", 128),
    publisher: boundedText(extension.publisher, "extension publisher", 128),
    version: boundedText(extension.version, "extension version", 128),
    displayName: boundedText(extension.displayName, "extension display name", 256),
    sourceKind: stringEnum(extension.sourceKind, "extension source kind", ["builtIn", "user"] as const),
    manifestJson: boundedText(extension.manifestJson, "extension manifest", 4 * 1024 * 1024),
    manifestSha256: boundedText(extension.manifestSha256, "extension manifest digest", 128),
  });
}

function normalizeDiagnostic(value: unknown): ExtensionDiagnostic {
  const diagnostic = record(value, "extension diagnostic");
  return Object.freeze({
    source: boundedText(diagnostic.source, "extension diagnostic source", 64),
    subject: diagnostic.subject === null ? undefined : boundedText(diagnostic.subject, "extension diagnostic subject", 256),
    code: stringEnum(diagnostic.code, "extension diagnostic code", ["sourceUnavailable", "invalidManifest", "duplicateExtension", "pathEscapesRoot", "resourceNotFound", "resourceTooLarge"] as const),
    message: boundedText(diagnostic.message, "extension diagnostic message", 512),
  });
}

function record(value: unknown, owner: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${owner} must be an object`);
  return value as Record<string, unknown>;
}

function array(value: unknown, owner: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`${owner} must be an array`);
  return value;
}

function boundedText(value: unknown, owner: string, maximum: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum) throw new TypeError(`${owner} is invalid`);
  return value;
}

function stringEnum<const T extends readonly string[]>(value: unknown, owner: string, values: T): T[number] {
  if (typeof value !== "string" || !values.includes(value)) throw new TypeError(`${owner} is invalid`);
  return value as T[number];
}

function nonNegativeSafeInteger(value: unknown, owner: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new TypeError(`${owner} is invalid`);
  return value as number;
}
