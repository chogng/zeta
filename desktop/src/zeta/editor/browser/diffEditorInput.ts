import { URI } from "../../base/common/uri.js";
import { type EditorInput } from "../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../workbench/browser/parts/editor/editorPane.js";

/** Persisted compatibility ID for the canonical diff editor. */
export const DIFF_EDITOR_ID = "zeta.editor.alpha-diff";
/** Persisted compatibility content type for explicit diff inputs. */
export const DIFF_EDITOR_CONTENT_TYPE = "application/vnd.zeta.alpha-diff";

/** One Workbench input that compares two ordinary text-resource editor inputs. */
export interface DiffEditorInput extends EditorInput {
  readonly contentType: typeof DIFF_EDITOR_CONTENT_TYPE;
  readonly original: EditorInput;
  readonly modified: EditorInput;
}

/** Creates a stable synthetic tab identity for an original/modified comparison. */
export function createDiffEditorInput(original: EditorInput, modified: EditorInput, label?: string): DiffEditorInput {
  assertTextResourceInput(original, "Diff original input");
  assertTextResourceInput(modified, "Diff modified input");
  if (label !== undefined && (typeof label !== "string" || label.trim().length === 0)) {
    throw new TypeError("Diff editor label must be a non-empty string");
  }
  const resource = URI.parse(`zeta-diff:/compare?original=${encodeURIComponent(original.resource.toString())}&modified=${encodeURIComponent(modified.resource.toString())}`);
  return Object.freeze({
    resource,
    contentType: DIFF_EDITOR_CONTENT_TYPE,
    original,
    modified,
    ...(label === undefined ? {} : { label: label.trim() }),
    readOnly: true,
  });
}

/** Narrows a generic Workbench editor input to the two-resource diff contract. */
export function isDiffEditorInput(input: EditorInput): input is DiffEditorInput {
  return input.contentType === DIFF_EDITOR_CONTENT_TYPE &&
    "original" in input &&
    "modified" in input &&
    isTextResourceInput(input.original) &&
    isTextResourceInput(input.modified);
}

/** Selects the dedicated diff pane only for explicit diff inputs. */
export function matchDiffEditor(input: EditorInput): EditorPaneMatch {
  return isDiffEditorInput(input) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

function assertTextResourceInput(value: unknown, owner: string): asserts value is EditorInput {
  if (!isTextResourceInput(value)) throw new TypeError(`${owner} requires an editor resource`);
}

function isTextResourceInput(value: unknown): value is EditorInput {
  return typeof value === "object" && value !== null &&
    "resource" in value &&
    typeof (value as EditorInput).resource?.toString === "function";
}
