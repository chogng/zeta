import { TextMateGrammarService, type ITextMateGrammarService } from "../common/textMateGrammarService.js";
import jsonGrammar from "./grammars/JSON.tmLanguage.json?raw";
import jsoncGrammar from "./grammars/JSONC.tmLanguage.json?raw";

/** Registers product-bundled TextMate grammars without exposing resource loading to common code. */
function registerBuiltinTextMateGrammars(service: ITextMateGrammarService): void {
  if (!service || typeof service !== "object" || typeof service.registerGrammar !== "function") {
    throw new TypeError("Built-in TextMate grammars require a TextMate grammar service");
  }
  const jsonRegistration = service.registerGrammar({
    scopeName: "source.json",
    languageId: "json",
    loadGrammar: () => jsonGrammar,
  });
  try {
    service.registerGrammar({
      scopeName: "source.json.comments",
      languageId: "jsonc",
      loadGrammar: () => jsoncGrammar,
    });
  } catch (error) {
    jsonRegistration.dispose();
    throw error;
  }
}

/** Browser catalog service containing Alpha's currently bundled product grammars. */
export class BrowserTextMateGrammarService extends TextMateGrammarService {
  constructor() {
    super();
    registerBuiltinTextMateGrammars(this);
  }
}
