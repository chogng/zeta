import { ChildProcessWithoutNullStreams } from "node:child_process";

type RpcId = number;
type RpcResponse = { id: RpcId; result?: unknown; error?: { code: number; message: string } };
type RpcNotification = { method: string; params: unknown };

/** Maintains response pairing for the one app-server stdio connection owned by Electron Main. */
export class JsonRpcConnection {
  #nextId = 1;
  #pending = new Map<RpcId, { resolve(value: unknown): void; reject(error: Error): void }>();
  #buffer = "";
  #notificationListeners = new Set<(notification: RpcNotification) => void>();

  constructor(private readonly process: ChildProcessWithoutNullStreams) {
    process.stdout.setEncoding("utf8");
    process.stdout.on("data", (chunk: string) => this.onData(chunk));
    process.once("exit", () => this.rejectAll(new Error("Zeta app-server exited")));
  }

  request<T>(method: string, params: unknown): Promise<T> {
    const id = this.#nextId++;
    const message = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    return new Promise<T>((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.process.stdin.write(`${message}\n`, "utf8");
    });
  }

  onNotification(listener: (notification: RpcNotification) => void): () => void {
    this.#notificationListeners.add(listener);
    return () => this.#notificationListeners.delete(listener);
  }

  private onData(chunk: string): void {
    this.#buffer += chunk;
    const lines = this.#buffer.split("\n");
    this.#buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line) continue;
      const message = JSON.parse(line) as RpcResponse | RpcNotification;
      if ("method" in message) {
        for (const listener of this.#notificationListeners) listener({ method: message.method, params: message.params });
        continue;
      }
      const response = message as RpcResponse;
      const pending = this.#pending.get(response.id);
      if (!pending) continue;
      this.#pending.delete(response.id);
      if (response.error) pending.reject(new Error(response.error.message));
      else pending.resolve(response.result);
    }
  }

  private rejectAll(error: Error): void {
    for (const pending of this.#pending.values()) pending.reject(error);
    this.#pending.clear();
  }
}
