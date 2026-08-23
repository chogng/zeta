export type SettingsItemKind = 'setting' | 'resource' | 'information';

/** Assigns the stable identity used to address an item inside the Settings editor. */
export function setSettingsItemIdentity(element: HTMLElement, id: string, kind: SettingsItemKind): void {
	if (!id || id !== id.trim()) throw new TypeError('Settings item IDs must be non-empty and must not have surrounding whitespace');
	element.dataset.settingsItemId = id;
	element.dataset.settingsItemKind = kind;
}
