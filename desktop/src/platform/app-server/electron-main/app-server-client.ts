import type {
  AppServerMethod,
  AppServerMethodDefinition,
  AppServerNotificationDefinition,
  AppServerNotificationMethod,
  MethodParams,
  MethodResult,
  NotificationParams,
  ServerNotification,
} from "../../../../generated/app-server/types.js";
import { APP_SERVER_NOTIFICATIONS } from "../../../../generated/app-server/types.js";
import {
  JsonRpcPeer,
  type RpcNotificationDefinition,
  type RpcRequestOptions,
} from "./json-rpc-peer.js";

/**
 * Restricts product calls to method and notification definitions emitted by the protocol generator.
 */
export class AppServerClient {
  constructor(readonly peer: JsonRpcPeer) {}

  request<M extends AppServerMethod>(
    definition: AppServerMethodDefinition<M>,
    params: MethodParams<M>,
    options?: RpcRequestOptions,
  ): Promise<MethodResult<M>> {
    return this.peer.request(definition, params, options);
  }

  onNotification<M extends AppServerNotificationMethod>(
    definition: AppServerNotificationDefinition<M>,
    listener: (params: NotificationParams<M>) => void,
  ): () => void {
    return this.peer.onNotification(definition, listener);
  }

  onAnyNotification(listener: (notification: ServerNotification) => void): () => void {
    const disposers = Object.values(APP_SERVER_NOTIFICATIONS).map((definition) =>
      this.peer.onNotification(
        definition as RpcNotificationDefinition<unknown>,
        (params) =>
          listener({ method: definition.method, params } as ServerNotification),
      ),
    );
    return () => {
      for (const dispose of disposers) dispose();
    };
  }

  diagnostics(): string {
    return this.peer.diagnostics();
  }

  close(): Promise<void> {
    return this.peer.close();
  }
}
