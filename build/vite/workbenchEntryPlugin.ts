import type { Connect, Plugin } from "vite";

interface WorkbenchEntryServer {
  readonly middlewares: {
    use(middleware: Connect.NextHandleFunction): void;
  };
}

export type ZetaWorkbenchEntryPlugin = Omit<Plugin, "configureServer"> & {
  readonly configureServer: (server: WorkbenchEntryServer) => void;
};

/**
 * Redirects the development server root to the shared Browser Workbench.
 */
export function workbenchEntryPlugin(): ZetaWorkbenchEntryPlugin {
  return {
    name: "zeta-workbench-entry",
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const method = request.method;
        const targetsRoot = request.url === "/" || request.url?.startsWith("/?");
        if ((method !== "GET" && method !== "HEAD") || !targetsRoot) {
          next();
          return;
        }

        response.statusCode = 302;
        response.setHeader("Cache-Control", "no-store");
        response.setHeader("Location", "/browser/workbench/workbench.html");
        response.end();
      });
    },
  };
}
