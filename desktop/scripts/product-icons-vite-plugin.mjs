import { dirname, extname, resolve } from "node:path";
import { syncProductIcons } from "./sync-product-icons.mjs";

const watchedEvents = new Set(["add", "change", "unlink"]);

/**
 * Keeps generated product-icon modules synchronized with their source SVGs
 * while Vite is running and reloads the Renderer after a successful update.
 */
export function productIconsPlugin(options = {}) {
  const sourceDirectory = resolve(options.sourceDirectory ?? resolve(import.meta.dirname, "../../resources/icons"));
  const outputFile = options.outputFile;
  const debounceMilliseconds = options.debounceMilliseconds ?? 50;
  let timer;
  let pending = Promise.resolve();

  return {
    name: "zeta-product-icons",
    configureServer(server) {
      server.watcher.add(sourceDirectory);
      server.watcher.on("all", (event, path) => {
        if (!watchedEvents.has(event) || !isDirectSvgChild(path, sourceDirectory)) {
          return;
        }
        clearTimeout(timer);
        timer = setTimeout(() => {
          pending = pending
            .catch(() => undefined)
            .then(() => syncProductIcons({ sourceDirectory, outputFile }))
            .then(() => server.ws.send({ type: "full-reload" }))
            .catch((error) => server.config.logger.error(error instanceof Error ? error.message : String(error), { error }));
        }, debounceMilliseconds);
      });
    },
  };
}

function isDirectSvgChild(path, sourceDirectory) {
  const resolvedPath = resolve(path);
  return extname(resolvedPath).toLowerCase() === ".svg" && samePath(dirname(resolvedPath), sourceDirectory);
}

function samePath(first, second) {
  if (process.platform === "win32") {
    return first.toLowerCase() === second.toLowerCase();
  }
  return first === second;
}
