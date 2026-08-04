import { APP_SERVER_METHODS, type DiffComputeParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_DIFF_INPUT_BYTES_PER_SIDE = 512 * 1024;

/** Exact-shape IPC route for the Rust-backed editor diff projection. */
export function diffIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [route({
    channel: "zeta:diff:compute",
    validate: diffComputeParams,
    invoke: params => supervisor.request(APP_SERVER_METHODS["diff/compute"], params),
  })];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: definition.validate,
    invoke: params => definition.invoke(params as P),
  };
}

function diffComputeParams(value: unknown): DiffComputeParams {
  const params = record(value, ["original", "modified"]);
  const original = boundedText(params.original, "original");
  const modified = boundedText(params.modified, "modified");
  return { original, modified };
}

function boundedText(value: unknown, name: string): string {
  const text = string(value, name);
  if (text.includes("\0") || new TextEncoder().encode(text).byteLength > MAX_DIFF_INPUT_BYTES_PER_SIDE) {
    throw new Error(`${name} must be NUL-free and no larger than ${MAX_DIFF_INPUT_BYTES_PER_SIDE} UTF-8 bytes`);
  }
  return text;
}
