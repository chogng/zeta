import type {
  IDimension,
} from "../../../../base/browser/geometry.js";
import type {
  IDisposable,
} from "../../../../base/common/lifecycle.js";
import type { URI } from "../../../../base/common/uri.js";
import type {
  IConfigurationService,
} from "../../../../platform/configuration/common/configuration.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import type { EditorInput } from "./editorInput.js";
import type { IDiffApi } from "../../../../platform/diff/common/diffApi.js";

export enum EditorPaneVisibility {
  Hidden,
  Visible,
}

/**
 * One editor implementation hosted by the central Editor Part.
 *
 * Implementations create their DOM exactly once in the supplied parent.
 * `setInput` may resolve asynchronously, must observe the abort signal, and
 * must reject when the input cannot be opened. The host owns the pane and
 * disposes it after hiding it.
 */
export interface IEditorPane extends IDisposable {
  readonly id: string;

  create(parent: HTMLElement): void;
  setInput(input: EditorInput, signal: AbortSignal): Promise<void>;
  clearInput(): void;
  layout(dimension: IDimension): void;
  setVisible(visibility: EditorPaneVisibility): void;
  focus(): void;
  /** Serializes and persists the active document to a new resource when supported. */
  saveAs?(resource: URI): Promise<void>;
}

export interface EditorPaneCreationOptions {
  readonly ownerDocument: Document;
  readonly configurationService?: IConfigurationService;
  readonly textFileService?: ITextFileService;
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
  readonly diffApi?: IDiffApi;
  readonly onSave?: () => Promise<void | boolean>;
}

export enum EditorPaneMatch {
  None,
  Optional,
  Default,
}

/**
 * Declares how one editor implementation is matched and constructed.
 *
 * Descriptors must keep `canOpen` pure. Product contribution modules register
 * descriptors before the Workbench creates its Editor Part.
 */
export interface IEditorPaneDescriptor {
  readonly id: string;
  readonly name: string;
  canOpen(input: EditorInput): EditorPaneMatch;
  create(options: EditorPaneCreationOptions): IEditorPane;
}
