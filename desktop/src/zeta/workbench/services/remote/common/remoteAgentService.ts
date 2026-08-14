import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { RemoteConnectionState } from "../../../../platform/remote/common/remote.js";

/** Exposes the active remote agent without leaking the backend transport API. */
export interface IRemoteAgentService {
  readonly connectionState: RemoteConnectionState | undefined;
  readonly onDidChangeConnectionState: Event<RemoteConnectionState>;
}

export const IRemoteAgentService = createServiceIdentifier<IRemoteAgentService>("remoteAgentService");
