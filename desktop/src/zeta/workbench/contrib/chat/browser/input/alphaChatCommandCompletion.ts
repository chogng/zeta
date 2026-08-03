import { LanguageCompletionItemKind } from "../../../../../editor/alpha/language/common/languageCompletions.js";
import { type LanguageCompletionProvider, type LanguageCompletionProviderRequest, type LanguageCompletionProviderResult } from "../../../../../editor/alpha/language/common/languageCompletionProviders.js";
import { TextPosition, TextRange } from "../../../../../editor/alpha/common/text.js";
import { type SlashCommandCatalog } from "../../common/slashCommands.js";

export const CHAT_INPUT_LANGUAGE_ID = "zeta-chat-input";

/** Adapts the Chat slash-command catalog to Alpha's completion contract. */
export function createAlphaChatCommandCompletionProvider(catalog: SlashCommandCatalog): LanguageCompletionProvider {
  return Object.freeze({
    id: "zeta.chat.commands",
    languageIds: Object.freeze([CHAT_INPUT_LANGUAGE_ID]),
    triggerCharacters: Object.freeze(["/"]),
    provideCompletions: (request: LanguageCompletionProviderRequest) => {
      if (request.position.lineIndex !== 0) return undefined;
      const line = request.snapshot.getText().split("\n", 1)[0] ?? "";
      const prefix = line.slice(0, request.position.columnIndex);
      if (!prefix.startsWith("/") || /\s/.test(prefix)) return emptyCompletionResult();
      const query = prefix.slice(1);
      const matches = catalog.matching(query);
      const range = TextRange.from(TextPosition.at(0, 0), request.position);
      return Object.freeze({
        items: Object.freeze(matches.map((command, index) => Object.freeze({
          id: command.name,
          label: `/${command.name}`,
          kind: LanguageCompletionItemKind.Function,
          range,
          insertText: `/${command.name} `,
          detail: command.description,
          filterText: `/${command.name}`,
          sortText: command.name,
          ...(index === 0 ? { preselect: true } : {}),
        }))),
        isIncomplete: true,
      });
    },
  });
}

function emptyCompletionResult(): LanguageCompletionProviderResult {
  return Object.freeze({ items: Object.freeze([]), isIncomplete: false });
}
