/** Cleanup handle that can cross Electron's contextBridge boundary. */
export interface ISandboxSubscription {
  dispose(): void;
}

/**
 * Minimal IPC capability exposed to a sandboxed renderer.
 *
 * Implementations must validate channel names and must not expose Electron
 * event objects or the underlying `ipcRenderer`.
 */
export interface ISandboxIpcRenderer {
  invoke(channel: string, params?: unknown): Promise<unknown>;
  on(
    channel: string,
    listener: (value: unknown) => void,
  ): ISandboxSubscription;
}

/** Read-only process metadata needed before the workbench starts. */
export interface ISandboxProcess {
  readonly platform: string;
  readonly arch: string;
}

/** Electron helpers that safely translate renderer-owned browser objects. */
export interface ISandboxWebUtils {
  getPathForFile(file: File): string;
}

/** Capabilities installed by the Electron sandbox preload. */
export interface ISandboxGlobals {
  readonly ipcRenderer: ISandboxIpcRenderer;
  readonly process: ISandboxProcess;
  readonly webUtils: ISandboxWebUtils;
}
