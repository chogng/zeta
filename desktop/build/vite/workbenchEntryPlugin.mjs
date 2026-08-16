/**
 * Redirects the development server root to the selected Browser Workbench.
 */
export function workbenchEntryPlugin(rendererEntry) {
  const entryPath = `/browser/workbench/${rendererEntry}.html`;

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
        response.setHeader("Location", entryPath);
        response.end();
      });
    },
  };
}
