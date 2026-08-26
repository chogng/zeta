import { toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import { type TextAreaEditContext } from "./textAreaEditContext.js";

/** Tracks textarea edit contexts for host integrations and diagnostics. */
class TextAreaEditContextRegistryImpl {

	private readonly contexts = new Map<HTMLElement, TextAreaEditContext>();

	register(element: HTMLElement, context: TextAreaEditContext): IDisposable {
		if (this.contexts.has(element)) throw new Error("A textarea edit context is already registered for this element");
		this.contexts.set(element, context);
		return toDisposable(() => {
			if (this.contexts.get(element) === context) this.contexts.delete(element);
		});
	}

	get(element: HTMLElement): TextAreaEditContext | undefined {
		return this.contexts.get(element);
	}
}

export const TextAreaEditContextRegistry = new TextAreaEditContextRegistryImpl();
