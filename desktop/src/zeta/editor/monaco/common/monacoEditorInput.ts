import type {
  EditorInput,
} from "../../../workbench/browser/parts/editor/editorInput.js";
import {
  EditorPaneMatch,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  ACADEMIC_DOCUMENT_CONTENT_TYPE,
} from "../../../product/common/documentTypes.js";

export const MONACO_EDITOR_ID = "zeta.editor.monaco";

const CONTENT_TYPE_LANGUAGES = new Map<string, string>([
  ["application/json", "json"],
  ["application/javascript", "javascript"],
  ["application/typescript", "typescript"],
  ["text/css", "css"],
  ["text/html", "html"],
  ["text/javascript", "javascript"],
  ["text/markdown", "markdown"],
  ["text/plain", "plaintext"],
  ["text/typescript", "typescript"],
]);

const EXTENSION_LANGUAGES = new Map<string, string>([
  [".c", "c"],
  [".cc", "cpp"],
  [".cpp", "cpp"],
  [".cs", "csharp"],
  [".css", "css"],
  [".go", "go"],
  [".html", "html"],
  [".java", "java"],
  [".js", "javascript"],
  [".json", "json"],
  [".jsx", "javascript"],
  [".md", "markdown"],
  [".py", "python"],
  [".rs", "rust"],
  [".sh", "shell"],
  [".sql", "sql"],
  [".toml", "ini"],
  [".ts", "typescript"],
  [".tsx", "typescript"],
  [".txt", "plaintext"],
  [".xml", "xml"],
  [".yaml", "yaml"],
  [".yml", "yaml"],
]);

/** Returns the product-level Monaco match without loading Monaco itself. */
export function matchMonacoEditor(input: EditorInput): EditorPaneMatch {
  if (input.contentType === ACADEMIC_DOCUMENT_CONTENT_TYPE) {
    return EditorPaneMatch.None;
  }
  if (
    input.contentType?.startsWith("text/") ||
    CONTENT_TYPE_LANGUAGES.has(input.contentType ?? "") ||
    extensionOf(input.resource.path) !== undefined
  ) {
    return EditorPaneMatch.Default;
  }
  return input.resource.scheme === "file"
    ? EditorPaneMatch.Optional
    : EditorPaneMatch.None;
}

/** Resolves a stable Monaco language identifier from content metadata. */
export function monacoLanguageForInput(input: EditorInput): string {
  const contentTypeLanguage = CONTENT_TYPE_LANGUAGES.get(
    input.contentType ?? "",
  );
  if (contentTypeLanguage) return contentTypeLanguage;
  return extensionOf(input.resource.path) ?? "plaintext";
}

function extensionOf(path: string): string | undefined {
  const fileName = path.slice(path.lastIndexOf("/") + 1).toLowerCase();
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0) return undefined;
  return EXTENSION_LANGUAGES.get(fileName.slice(dot));
}
