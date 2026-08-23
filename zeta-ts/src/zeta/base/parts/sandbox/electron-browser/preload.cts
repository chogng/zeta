(function () {
  const { contextBridge, ipcRenderer, webUtils } =
    require("electron") as typeof import("electron");
  type ISandboxGlobals =
    import("../common/sandboxTypes.js").ISandboxGlobals;

  const validateChannel = (channel: string): string => {
    if (!channel?.startsWith("zeta:")) {
      throw new Error(`Unsupported IPC channel '${channel}'`);
    }
    return channel;
  };

  const globals: ISandboxGlobals = {
    ipcRenderer: {
      invoke: (channel, params) =>
        ipcRenderer.invoke(validateChannel(channel), params),
      on: (channel, listener) => {
        const validatedChannel = validateChannel(channel);
        const handler = (
          _event: Electron.IpcRendererEvent,
          value: unknown,
        ): void => listener(value);
        ipcRenderer.on(validatedChannel, handler);
        return {
          dispose: () =>
            ipcRenderer.removeListener(validatedChannel, handler),
        };
      },
    },
    process: {
      platform: process.platform,
      arch: process.arch,
    },
    webUtils: {
      getPathForFile: (file) => webUtils.getPathForFile(file),
    },
  };

  contextBridge.exposeInMainWorld("zeta", globals);
})();
