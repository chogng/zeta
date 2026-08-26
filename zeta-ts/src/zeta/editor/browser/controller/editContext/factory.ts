import { EditContext, type EditContextOptions } from "./editContext.js";
import { createNativeEditContext, supportsNativeEditContext } from "./native/editContextFactory.js";
import { TextAreaEditContext } from "./textArea/textAreaEditContext.js";

/** Creates the best browser editing surface available for one editor. */
export function createEditContext(
	container: HTMLElement,
	options: EditContextOptions = {},
): EditContext {
	if (supportsNativeEditContext(container)) {
		try {
			return createNativeEditContext(container, options);
		} catch {
			// A partially implemented browser API is treated like an unsupported one.
		}
	}
	return new TextAreaEditContext(container, options);
}
