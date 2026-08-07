import { APP_SERVER_METHODS, type SyntaxAnalyzeParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_SYNTAX_INPUT_BYTES = 4 * 1024 * 1024;
const SUPPORTED_LANGUAGES = new Set(["json", "jsonc", "rust", "shell"]);

/** Exact-shape IPC route for the Rust-backed source syntax projection. */
export function syntaxIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [route({
    channel: "zeta:syntax:analyze",
    validate: syntaxAnalyzeParams,
    invoke: params => supervisor.request(APP_SERVER_METHODS["syntax/analyze"], params),
  })];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: definition.validate,
    invoke: params => definition.invoke(params as P),
  };
}

function syntaxAnalyzeParams(value: unknown): SyntaxAnalyzeParams {
  const params = record(value, ["language", "revision", "text"]);
  const language = string(params.language, "language");
  if (!SUPPORTED_LANGUAGES.has(language)) {
    throw new Error("language must be one of json, jsonc, rust, or shell");
  }
  const text = string(params.text, "text");
  if (new TextEncoder().encode(text).byteLength > MAX_SYNTAX_INPUT_BYTES) {
    throw new Error(`text must not exceed ${MAX_SYNTAX_INPUT_BYTES} UTF-8 bytes`);
  }
  return {
    language: language as SyntaxAnalyzeParams["language"],
    revision: nonNegativeInteger(params.revision, "revision"),
    text,
  };
}
