import { isCancellationError } from "../../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type Event } from "../../../../base/common/event.js";
import { type SyntaxService } from "../../../common/languages/syntax/syntaxService.js";

/** Schedules syntax lanes while the selected Workbench mode's language support changes. */
export class LanguageAnalysisController extends DisposableOwner {
  private generation = 0;
  private disposed = false;

  constructor(private readonly syntax: SyntaxService, private readonly languageId: string, whenLanguageSupportReady: () => Promise<unknown>, onDidChangeLanguageSupport: Event<void> | undefined, private readonly onLanguageError: (error: unknown) => void) {
    super();
    const schedule = () => {
      const generation = ++this.generation;
      queueMicrotask(() => void this.run(generation, whenLanguageSupportReady));
    };
    this.own(syntax.tokens.textModel.onDidChange(schedule));
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
      await this.syntax.requestAll(this.languageId);
    } catch (error) {
      if (this.disposed || generation !== this.generation || isCancellationError(error) || isAbortError(error)) return;
      this.onLanguageError(error);
    }
  }
}

function isAbortError(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}
