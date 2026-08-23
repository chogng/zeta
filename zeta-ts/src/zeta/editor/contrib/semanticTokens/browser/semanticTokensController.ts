import { isCancellationError } from "../../../../base/common/cancellation.js";
import { type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type SemanticTokensService } from "../common/semanticTokens.js";

/** Refreshes full semantic tokens while the document and provider set remain current. */
export class SemanticTokensController extends DisposableOwner {
  private generation = 0;
  private disposed = false;

  constructor(private readonly service: SemanticTokensService, private readonly languageId: string, whenLanguageSupportReady: () => Promise<unknown>, onDidChangeLanguageSupport: Event<void> | undefined, private readonly onLanguageError: (error: unknown) => void) {
    super();
    const schedule = () => {
      const generation = ++this.generation;
      queueMicrotask(() => void this.run(generation, whenLanguageSupportReady));
    };
    this.own(service.tokens.textModel.onDidChange(schedule));
    if (onDidChangeLanguageSupport) this.own(onDidChangeLanguageSupport(schedule));
    this.defer(() => {
      this.disposed = true;
      this.generation += 1;
    });
    schedule();
  }

  private async run(generation: number, whenLanguageSupportReady: () => Promise<unknown>): Promise<void> {
    try {
      await whenLanguageSupportReady();
      if (this.disposed || generation !== this.generation) return;
      await this.service.requestTokens(this.languageId);
    } catch (error) {
      if (this.disposed || generation !== this.generation || isCancellationError(error) || isAbortError(error)) return;
      this.onLanguageError(error);
    }
  }
}

function isAbortError(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}
