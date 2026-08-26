import { toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import type { NativeEditContext } from "./nativeEditContext.js";

type NativeEditContextOwner = string | HTMLElement;

/**
 * Finds an active native edit context without making clipboard or host code
 * depend on the concrete input controller instance.
 */
class NativeEditContextRegistryImpl {
	private readonly byId = new Map<string, NativeEditContext>();
	private readonly byElement = new WeakMap<HTMLElement, NativeEditContext>();

	register(owner: NativeEditContextOwner, context: NativeEditContext): IDisposable {
		if (typeof owner === "string") {
			const previous = this.byId.get(owner);
			if (previous && previous !== context) throw new Error(`Native EditContext owner '${owner}' is already registered`);
			this.byId.set(owner, context);
		} else {
			const previous = this.byElement.get(owner);
			if (previous && previous !== context) throw new Error("Native EditContext element is already registered");
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

	get(owner: NativeEditContextOwner): NativeEditContext | undefined {
		return typeof owner === "string" ? this.byId.get(owner) : this.byElement.get(owner);
	}
}

export const NativeEditContextRegistry = new NativeEditContextRegistryImpl();
