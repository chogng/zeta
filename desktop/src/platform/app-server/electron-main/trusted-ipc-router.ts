export interface IpcMainInvokeEventLike {
  readonly sender: {
    readonly mainFrame: unknown;
  };
  readonly senderFrame: {
    readonly url: string;
  } | null;
}

export interface IpcMainLike {
  handle(
    channel: string,
    listener: (event: IpcMainInvokeEventLike, params: unknown) => unknown,
  ): void;
  removeHandler(channel: string): void;
}

export interface IpcRoute<P, R> {
  readonly channel: string;
  readonly validate: (value: unknown) => P;
  readonly invoke: (params: P) => R | Promise<R>;
}

export interface TrustedIpcTarget {
  readonly webContents: {
    readonly mainFrame: unknown;
  };
  readonly allowedEntryUrls: ReadonlySet<string>;
}

/**
 * Registers finite IPC routes with one shared sender, main-frame, URL, and params gate.
 */
export function registerTrustedIpcRoutes(
  ipcMain: IpcMainLike,
  target: TrustedIpcTarget,
  routes: readonly IpcRoute<unknown, unknown>[],
): () => void {
  const channels = new Set<string>();
  for (const route of routes) {
    if (channels.has(route.channel)) {
      throw new Error(`Duplicate IPC route: ${route.channel}`);
    }
    channels.add(route.channel);
  }
  const registered: string[] = [];
  try {
    for (const route of routes) {
      ipcMain.handle(route.channel, (event, rawParams) => {
        requireTrustedSender(event, target);
        return route.invoke(route.validate(rawParams));
      });
      registered.push(route.channel);
    }
  } catch (error) {
    for (const channel of registered) ipcMain.removeHandler(channel);
    throw error;
  }
  return () => {
    for (const channel of channels) ipcMain.removeHandler(channel);
  };
}

export function requireTrustedSender(
  event: IpcMainInvokeEventLike,
  target: TrustedIpcTarget,
): void {
  if (event.sender !== target.webContents) {
    throw new Error("Untrusted renderer IPC sender");
  }
  if (!event.senderFrame || event.senderFrame !== event.sender.mainFrame) {
    throw new Error("Renderer IPC must originate from the main frame");
  }
  if (!target.allowedEntryUrls.has(normalizeEntryUrl(event.senderFrame.url))) {
    throw new Error("Renderer IPC URL is not allowed");
  }
}

export function normalizeEntryUrl(value: string): string {
  return new URL(value).href;
}
