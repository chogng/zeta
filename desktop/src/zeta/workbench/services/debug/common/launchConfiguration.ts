import { parseJsonc } from "../../../../base/common/jsonc.js";
import { type IDebugConfiguration } from "./debugService.js";

const MAX_CONFIGURATIONS = 64;

/** Parses Zeta's explicit generic-adapter extension of VS Code launch.json. */
export function parseLaunchConfigurations(source: string): readonly IDebugConfiguration[] {
  const document = record(parseJsonc(source, ".vscode/launch.json"), ".vscode/launch.json");
  if (document.version !== "0.2.0") throw new TypeError(".vscode/launch.json version must be '0.2.0'");
  if (!Array.isArray(document.configurations)) throw new TypeError(".vscode/launch.json configurations must be an array");
  if (document.configurations.length > MAX_CONFIGURATIONS) throw new RangeError(`.vscode/launch.json cannot contain more than ${MAX_CONFIGURATIONS} configurations`);
  return Object.freeze(document.configurations.map((value, index) => parseConfiguration(value, index)));
}

function parseConfiguration(value: unknown, index: number): IDebugConfiguration {
  const input = record(value, `configurations[${index}]`);
  const name = boundedString(input.name, `configurations[${index}].name`, 256);
  const type = boundedString(input.type, `configurations[${index}].type`, 128);
  const request = input.request;
  if (request !== "launch" && request !== "attach") throw new TypeError(`configurations[${index}].request must be 'launch' or 'attach'`);
  const adapter = record(input.debugAdapter, `configurations[${index}].debugAdapter`);
  const program = boundedString(adapter.program, `configurations[${index}].debugAdapter.program`, 4096);
  const argumentsList = adapter.args === undefined ? [] : array(adapter.args, `configurations[${index}].debugAdapter.args`).map((argument, argumentIndex) => boundedString(argument, `configurations[${index}].debugAdapter.args[${argumentIndex}]`, 4096));
  if (argumentsList.length > 128) throw new RangeError(`configurations[${index}].debugAdapter.args cannot contain more than 128 values`);
  const launchArguments = Object.fromEntries(Object.entries(input).filter(([key]) => !["name", "type", "request", "debugAdapter"].includes(key)));
  ensureJsonCompatible(launchArguments, `configurations[${index}]`, 0);
  return Object.freeze({ id: `launch:${index}:${stableId(name)}`, name, type, request, adapter: Object.freeze({ program, arguments: Object.freeze(argumentsList) }), arguments: Object.freeze(launchArguments) });
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${path} must be an object`);
  return value as Record<string, unknown>;
}

function array(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`${path} must be an array`);
  return value;
}

function boundedString(value: unknown, path: string, maximum: number): string {
  if (typeof value !== "string" || !value.trim() || value.length > maximum || value.includes("\0")) throw new TypeError(`${path} must contain 1 to ${maximum} characters`);
  return value.trim();
}

function ensureJsonCompatible(value: unknown, path: string, depth: number): void {
  if (depth > 64) throw new RangeError(`${path} exceeds the supported nesting depth`);
  if (value === null || typeof value === "string" || typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value))) return;
  if (Array.isArray(value)) { value.forEach((item, index) => ensureJsonCompatible(item, `${path}[${index}]`, depth + 1)); return; }
  if (typeof value === "object") { Object.entries(value as Record<string, unknown>).forEach(([key, item]) => ensureJsonCompatible(item, `${path}.${key}`, depth + 1)); return; }
  throw new TypeError(`${path} must be JSON-compatible`);
}

function stableId(value: string): string {
  return value.toLocaleLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 80) || "configuration";
}
