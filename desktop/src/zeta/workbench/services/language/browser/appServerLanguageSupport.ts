export const APP_SERVER_LANGUAGE_IDS = Object.freeze(["javascript", "javascriptreact", "json", "jsonc", "rust", "shell", "typescript", "typescriptreact"]);

const APP_SERVER_LANGUAGE_ID_SET = new Set<string>(APP_SERVER_LANGUAGE_IDS);

export function isAppServerLanguageId(languageId: string): boolean {
  return APP_SERVER_LANGUAGE_ID_SET.has(languageId);
}
