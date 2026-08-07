import { APP_SERVER_METHODS, type DocumentCollaborationOpenParams, type DocumentCollaborationSubmitParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_DOCUMENT_BYTES = 4 * 1024 * 1024;
const MAX_TRANSACTION_BYTES = 1_048_576;

/** Exact-shape IPC routes for the server-ordered Gama collaboration protocol. */
export function documentCollaborationIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:document:collaboration:open",
      validate: documentCollaborationOpenParams,
      invoke: params => supervisor.request(APP_SERVER_METHODS["document/collaboration/open"], params),
    }),
    route({
      channel: "zeta:document:collaboration:submit",
      validate: documentCollaborationSubmitParams,
      invoke: params => supervisor.request(APP_SERVER_METHODS["document/collaboration/submit"], params),
    }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: value => definition.validate(value) as unknown,
    invoke: params => definition.invoke(params as P),
  };
}

function documentCollaborationOpenParams(value: unknown): DocumentCollaborationOpenParams {
  const params = record(value, ["roomId", "clientId", "schemaId", "document"]);
  const document = boundedString(params.document, "document", MAX_DOCUMENT_BYTES);
  return {
    ...(params.roomId === undefined ? {} : { roomId: nonEmptyString(params.roomId, "roomId") }),
    clientId: nonEmptyString(params.clientId, "clientId"),
    schemaId: nonEmptyString(params.schemaId, "schemaId"),
    document,
  };
}

function documentCollaborationSubmitParams(value: unknown): DocumentCollaborationSubmitParams {
  const params = record(value, ["roomId", "clientId", "sequence", "baseVersion", "transaction", "document"]);
  return {
    roomId: nonEmptyString(params.roomId, "roomId"),
    clientId: nonEmptyString(params.clientId, "clientId"),
    sequence: collaborationInteger(params.sequence, "sequence", 1),
    baseVersion: collaborationInteger(params.baseVersion, "baseVersion", 0),
    transaction: boundedString(params.transaction, "transaction", MAX_TRANSACTION_BYTES),
    document: boundedString(params.document, "document", MAX_DOCUMENT_BYTES),
  };
}

function boundedString(value: unknown, name: string, maximumBytes: number): string {
  const text = string(value, name);
  if (new TextEncoder().encode(text).byteLength > maximumBytes) throw new Error(`${name} must not exceed ${maximumBytes} UTF-8 bytes`);
  return text;
}

function collaborationInteger(value: unknown, name: string, minimum: number): number {
  if (!Number.isSafeInteger(value) || (value as number) < minimum) {
    throw new Error(`${name} must be a safe integer greater than or equal to ${minimum}`);
  }
  return value as number;
}
