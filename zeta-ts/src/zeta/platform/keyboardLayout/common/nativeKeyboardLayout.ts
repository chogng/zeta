import type { IKeyboardLayoutDefinition, IKeyboardLayoutProvider } from './keyboardLayout.js';
import { validateKeyboardLayoutDefinition } from './keyboardLayoutValidation.js';

export const NATIVE_KEYBOARD_LAYOUT_READ_CHANNEL = 'zeta:keyboard-layout:read';
export const NATIVE_KEYBOARD_LAYOUT_CHANGED_CHANNEL = 'zeta:keyboard-layout:changed';

export interface INativeKeyboardLayoutApi extends IKeyboardLayoutProvider {}

export function validateNativeKeyboardLayout(value: unknown): IKeyboardLayoutDefinition | undefined {
	return validateKeyboardLayoutDefinition(value, 'native');
}
