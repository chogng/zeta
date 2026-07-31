import { type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { assertLanguageId } from "./languageId.js";
import { LanguageLexicalLineScanner } from "./languageLexicalLineScanner.js";

const ECMASCRIPT_LANGUAGE_IDS = new Set(["typescript", "typescriptreact", "javascript", "javascriptreact"]);
const JSON_LANGUAGE_IDS = new Set(["json", "jsonc"]);
const ECMASCRIPT_KEYWORDS = Object.freeze([
  "as", "async", "await", "break", "case", "catch", "class", "const", "continue",
  "debugger", "declare", "default", "delete", "do", "else", "enum", "export",
  "extends", "false", "finally", "for", "from", "function", "get", "if", "implements",
  "import", "in", "infer", "instanceof", "interface", "keyof", "let", "module",
  "namespace", "new", "null", "of", "package", "private", "protected", "public",
  "readonly", "return", "satisfies", "set", "static", "super", "switch", "this",
  "throw", "true", "try", "type", "typeof", "undefined", "using", "var", "void",
  "while", "with", "yield",
]);
const JSON_KEYWORDS = Object.freeze(["false", "null", "true"]);

export function createLanguageLexicalLineScanner(languageId: string, configuration: ResolvedLanguageConfiguration): LanguageLexicalLineScanner {
  assertLanguageId(languageId);
  if (configuration.languageId !== languageId) {
    throw new Error("Language lexical configuration identity does not match its language");
  }
  if (ECMASCRIPT_LANGUAGE_IDS.has(languageId)) {
    return new LanguageLexicalLineScanner({
      comments: configuration.comments,
      brackets: configuration.brackets,
      keywords: ECMASCRIPT_KEYWORDS,
      stringQuotes: ["'", "\""],
      multilineStringQuote: "`",
    });
  }
  if (JSON_LANGUAGE_IDS.has(languageId)) {
    return new LanguageLexicalLineScanner({
      comments: configuration.comments,
      brackets: configuration.brackets,
      keywords: JSON_KEYWORDS,
      stringQuotes: ["\""],
    });
  }
  return new LanguageLexicalLineScanner({
    comments: configuration.comments,
    brackets: configuration.brackets,
    keywords: [],
    stringQuotes: ["'", "\""],
  });
}
