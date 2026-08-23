import type { URI } from "../../base/common/uri.js";

/** Resource identity and presentation hints accepted by an editor surface. */
export interface EditorResourceInput {
  readonly resource: URI;
  readonly label?: string;
  readonly languageId?: string;
  readonly readOnly?: boolean;
  readonly initialText?: string;
}
