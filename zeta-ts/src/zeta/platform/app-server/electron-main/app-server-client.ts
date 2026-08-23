import type {
  AppServerMethod,
  AppServerMethodDefinition,
  AppServerNotificationDefinition,
  AppServerNotificationMethod,
  MethodParams,
  MethodResult,
  NotificationParams,
  ServerNotification,
} from "../../../../../generated/app-server/types.js";
import { APP_SERVER_NOTIFICATIONS } from "../../../../../generated/app-server/types.js";
import {
  DisposableStore,
  type IDisposable,
  markAsDisposed,
  setDisposableOwner,
  trackDisposable,
} from "../../../base/common/lifecycle.js";
import {
  JsonRpcPeer,
  type RpcNotificationDefinition,
  type RpcRequestOptions,
} from "./json-rpc-peer.js";

/**
 * Restricts product calls to method and notification definitions emitted by the protocol generator.
 */
export class AppServerClient implements IDisposable {
  constructor(readonly peer: JsonRpcPeer) {
    trackDisposable(this);
    setDisposableOwner(peer, this);
  }

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
  ): IDisposable {
    return this.peer.onNotification(definition, listener);
  }

  onAnyNotification(
    listener: (notification: ServerNotification) => void,
  ): IDisposable {
    const subscriptions = new DisposableStore();
    try {
      for (const definition of Object.values(APP_SERVER_NOTIFICATIONS)) {
        subscriptions.add(this.peer.onNotification(
        definition as RpcNotificationDefinition<unknown>,
        (params) =>
          listener({ method: definition.method, params } as ServerNotification),
        ));
      }
    } catch (error) {
      subscriptions.dispose();
      throw error;
    }
    return subscriptions;
  }

  diagnostics(): string {
    return this.peer.diagnostics();
  }

  async close(): Promise<void> {
    try {
      await this.peer.close();
    } finally {
      markAsDisposed(this);
    }
  }

  dispose(): void {
    try {
      this.peer.dispose();
    } finally {
      markAsDisposed(this);
    }
  }

  [Symbol.dispose](): void {
    this.dispose();
  }
}
