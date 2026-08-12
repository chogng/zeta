import { DisposableStore, type IDisposable } from "../../../base/common/lifecycle.js";
import { LanguageRegistry, type LanguageDescription } from "./languageRegistry.js";

const BUILTIN_LANGUAGE_DESCRIPTIONS: readonly LanguageDescription[] = Object.freeze([
  { id: "c", extensions: [".c"] },
  { id: "cpp", extensions: [".cc", ".cpp"] },
  { id: "csharp", extensions: [".cs"] },
  { id: "css", extensions: [".css"], mimetypes: ["text/css"] },
  { id: "go", extensions: [".go"] },
  { id: "html", extensions: [".html"], mimetypes: ["text/html"] },
  { id: "java", extensions: [".java"] },
  { id: "javascript", extensions: [".js", ".mjs"], mimetypes: ["application/javascript", "text/javascript"] },
  { id: "javascriptreact", extensions: [".jsx"] },
  { id: "json", extensions: [".json"], mimetypes: ["application/json"] },
  { id: "jsonc", extensions: [".jsonc"] },
  { id: "markdown", extensions: [".md"], mimetypes: ["text/markdown"] },
  { id: "python", extensions: [".py"] },
  { id: "rust", extensions: [".rs"] },
  { id: "shell", extensions: [".sh"] },
  { id: "sql", extensions: [".sql"] },
  { id: "typescript", extensions: [".ts"], mimetypes: ["application/typescript", "text/typescript"] },
  { id: "typescriptreact", extensions: [".tsx"] },
  { id: "plaintext", extensions: [".txt"], mimetypes: ["text/plain"] },
  { id: "xml", extensions: [".xml"] },
  { id: "yaml", extensions: [".yaml", ".yml"] },
  { id: "ini", extensions: [".toml"] },
]);

/** Registers the product's baseline language associations before extensions load. */
export function registerBuiltinLanguageDescriptions(registry: LanguageRegistry): IDisposable {
  const registrations = new DisposableStore();
  for (const description of BUILTIN_LANGUAGE_DESCRIPTIONS) registrations.add(registry.register(description));
  return registrations;
}
