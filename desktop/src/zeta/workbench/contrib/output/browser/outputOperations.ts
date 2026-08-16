import { URI } from "../../../../base/common/uri.js";
import type { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import type { IWorkbenchWindowService } from "../../../browser/window.js";
import type { IOutputChannel } from "../../../services/output/common/outputService.js";

/** Opens a read-only point-in-time snapshot of one Output channel. */
export async function openOutputChannelInEditor(channel: IOutputChannel, editorPart: IEditorPart): Promise<void> {
  const name = safeOutputFileName(channel.label);
  const resource = URI.parse(`untitled:/Output-${encodeURIComponent(name)}-${Date.now()}.log`);
  await editorPart.openEditor({ resource, contentType: "text/plain", ...(channel.descriptor.languageId ? { languageId: channel.descriptor.languageId } : {}), label: `${channel.label}.log`, readOnly: true, initialText: channel.getText() });
}

/** Downloads the complete retained content of one Output channel. */
export function exportOutputChannel(channel: IOutputChannel, windowService: IWorkbenchWindowService): void {
  const document = windowService.root.ownerDocument;
  const targetWindow = document.defaultView;
  if (!targetWindow) throw new Error("Output export requires a browser window");
  const url = targetWindow.URL.createObjectURL(new targetWindow.Blob([channel.getText()], { type: "text/plain;charset=utf-8" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${safeOutputFileName(channel.label)}.log`;
  anchor.click();
  targetWindow.setTimeout(() => targetWindow.URL.revokeObjectURL(url), 0);
}

export function safeOutputFileName(label: string): string {
  return label.replace(/[\\/:*?"<>|\u0000-\u001F]/g, "-").trim() || "Output";
}
