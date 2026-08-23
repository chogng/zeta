import type { Event } from "../../../base/common/event.js";
import type {
	Keybinding,
	KeybindingEvent,
	ResolvedKeybinding,
} from "../../../base/common/keybindings.js";
import {
	createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

/** Identifies the keyboard layout currently used to resolve shortcut labels. */
export interface IKeyboardLayoutInfo {
	readonly id: string;
	readonly label: string;
	readonly source: "browser" | "fallback";
}

/**
 * Maps keybindings and native event data for one active keyboard layout.
 *
 * Implementations preserve physical dispatch identity while applying
 * layout-aware labels and host modifier conventions.
 */
export interface IKeyboardMapper {
	resolveKeybinding(keybinding: Keybinding): ResolvedKeybinding;
}

/** Supplies the active keyboard mapper to the Workbench keybinding service. */
export interface IKeyboardLayoutService {
	readonly onDidChangeKeyboardLayout: Event<void>;

	getCurrentKeyboardLayout(): IKeyboardLayoutInfo;
	getKeyboardMapper(): IKeyboardMapper;
	validateCurrentKeyboardMapping(event: KeybindingEvent): void;
	refreshKeyboardLayout(): Promise<void>;
}

export const IKeyboardLayoutService =
	createServiceIdentifier<IKeyboardLayoutService>("keyboardLayoutService");
