import {
  DisposableStore,
  type IDisposable,
  toDisposable,
} from "../common/lifecycle.js";
import { addDisposableListener } from "./dom.js";
import { mainWindow } from "./window.js";

export type FileSelection = "single" | "multiple";

export interface FilePickerOptions {
  readonly document?: Document;
  readonly accept?: readonly string[];
  readonly selection?: FileSelection;
  readonly directory?: boolean;
  readonly signal?: AbortSignal;
}

/** Opens a native browser file picker and resolves undefined when cancelled. */
export function pickFiles(
  options: FilePickerOptions = {},
): Promise<readonly File[] | undefined> {
  const ownerDocument = options.document ?? mainWindow.document;
  const input = ownerDocument.createElement("input");
  input.type = "file";
  input.hidden = true;
  input.multiple = options.selection === "multiple";
  input.accept = options.accept?.join(",") ?? "";
  if (options.directory) input.setAttribute("webkitdirectory", "");
  ownerDocument.body.append(input);

  return new Promise((resolve) => {
    const registrations = new DisposableStore();
    let settled = false;
    const finish = (files: readonly File[] | undefined): void => {
      if (settled) return;
      settled = true;
      try {
        resolve(files);
      } finally {
        registrations.dispose();
        input.remove();
      }
    };

    registrations.add(addDisposableListener(input, "change", () =>
      finish(input.files ? [...input.files] : undefined),
    ));
    registrations.add(addDisposableListener(input, "cancel", () =>
      finish(undefined),
    ));

    const signal = options.signal;
    if (signal?.aborted) {
      finish(undefined);
      return;
    }
    if (signal) {
      const onAbort = (): void => finish(undefined);
      signal.addEventListener("abort", onAbort, { once: true });
      registrations.add(toDisposable(() =>
        signal.removeEventListener("abort", onAbort),
      ));
    }

    input.click();
  });
}

/** Downloads a Blob or existing URL under an explicit filename. */
export function triggerDownload(
  source: Blob | URL,
  name: string,
  ownerDocument: Document = mainWindow.document,
): void {
  const objectUrl = source instanceof Blob
    ? URL.createObjectURL(source)
    : undefined;
  const link = ownerDocument.createElement("a");
  link.download = name;
  link.href = objectUrl ?? source.toString();
  link.rel = "noopener";
  link.style.display = "none";
  ownerDocument.body.append(link);
  link.click();
  link.remove();
  if (objectUrl) {
    const targetWindow = ownerDocument.defaultView ?? mainWindow;
    targetWindow.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
  }
}

/** Creates a temporary object URL with an explicit disposable lifetime. */
export function createObjectUrl(blob: Blob): {
  readonly url: URL;
  readonly registration: IDisposable;
} {
  const value = URL.createObjectURL(blob);
  return {
    url: new URL(value),
    registration: toDisposable(() => URL.revokeObjectURL(value)),
  };
}
