import CssWorker from
  "monaco-editor/language/css/css.worker.js?worker";
import EditorWorker from
  "monaco-editor/editor/editor.worker.js?worker";
import HtmlWorker from
  "monaco-editor/language/html/html.worker.js?worker";
import JsonWorker from
  "monaco-editor/language/json/json.worker.js?worker";
import TypeScriptWorker from
  "monaco-editor/language/typescript/ts.worker.js?worker";

interface MonacoWorkerEnvironment {
  getWorker(moduleId: string, label: string): Worker;
}

type MonacoGlobal = typeof globalThis & {
  MonacoEnvironment?: MonacoWorkerEnvironment;
};

const monacoGlobal = globalThis as MonacoGlobal;

monacoGlobal.MonacoEnvironment ??= {
  getWorker(_moduleId, label) {
    if (label === "json") return new JsonWorker();
    if (label === "css" || label === "scss" || label === "less") {
      return new CssWorker();
    }
    if (label === "html" || label === "handlebars" || label === "razor") {
      return new HtmlWorker();
    }
    if (label === "typescript" || label === "javascript") {
      return new TypeScriptWorker();
    }
    return new EditorWorker();
  },
};
