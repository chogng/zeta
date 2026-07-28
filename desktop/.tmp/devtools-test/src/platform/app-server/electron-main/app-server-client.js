import { APP_SERVER_NOTIFICATIONS } from "../../../../generated/app-server/types.js";
import { DisposableStore, markAsDisposed, setDisposableOwner, trackDisposable, } from "../../../base/common/lifecycle.js";
/**
 * Restricts product calls to method and notification definitions emitted by the protocol generator.
 */
export class AppServerClient {
    peer;
    constructor(peer) {
        this.peer = peer;
        trackDisposable(this);
        setDisposableOwner(peer, this);
    }
    request(definition, params, options) {
        return this.peer.request(definition, params, options);
    }
    onNotification(definition, listener) {
        return this.peer.onNotification(definition, listener);
    }
    onAnyNotification(listener) {
        const subscriptions = new DisposableStore();
        try {
            for (const definition of Object.values(APP_SERVER_NOTIFICATIONS)) {
                subscriptions.add(this.peer.onNotification(definition, (params) => listener({ method: definition.method, params })));
            }
        }
        catch (error) {
            subscriptions.dispose();
            throw error;
        }
        return subscriptions;
    }
    diagnostics() {
        return this.peer.diagnostics();
    }
    async close() {
        try {
            await this.peer.close();
        }
        finally {
            markAsDisposed(this);
        }
    }
    dispose() {
        try {
            this.peer.dispose();
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
