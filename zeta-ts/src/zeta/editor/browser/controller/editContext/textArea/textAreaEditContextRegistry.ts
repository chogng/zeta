import { toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import { type TextAreaEditContext } from "./textAreaEditContext.js";

type TextAreaEditContextOwner = string | HTMLElement;

/** Tracks textarea edit contexts for host integrations and diagnostics. */
class TextAreaEditContextRegistryImpl {
	private readonly byId = new Map<string, TextAreaEditContext>();
	private readonly byElement = new WeakMap<HTMLElement, TextAreaEditContext>();

	register(owner: TextAreaEditContextOwner, context: TextAreaEditContext): IDisposable {
		if (typeof owner === "string") {
			const previous = this.byId.get(owner);
			if (previous && previous !== context) throw new Error(`Textarea edit-context owner '${owner}' is already registered`);
			this.byId.set(owner, context);
		} else {
			const previous = this.byElement.get(owner);
			if (previous && previous !== context) throw new Error("A textarea edit context is already registered for this element");
			this.byElement.set(owner, context);
		}
		return toDisposable(() => {
			if (typeof owner === "string") {
				if (this.byId.get(owner) === context) this.byId.delete(owner);
			} else if (this.byElement.get(owner) === context) {
				this.byElement.delete(owner);
			}
		});
	}

	get(owner: TextAreaEditContextOwner): TextAreaEditContext | undefined {
		return typeof owner === "string" ? this.byId.get(owner) : this.byElement.get(owner);
	}
}

export const TextAreaEditContextRegistry = new TextAreaEditContextRegistryImpl();
