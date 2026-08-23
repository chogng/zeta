import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** Writes explicit user-facing text through the current host clipboard. */
export interface IClipboardService {
	writeText(value: string): Promise<void>;
}

export const IClipboardService = createServiceIdentifier<IClipboardService>("clipboardService");
