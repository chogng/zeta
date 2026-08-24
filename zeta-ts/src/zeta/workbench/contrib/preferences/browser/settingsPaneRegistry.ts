import type { IDisposable } from '../../../../base/common/lifecycle.js';

export interface SettingsPaneNavigationTarget {
	readonly title: string;
	readonly description: string;
}

export interface SettingsPane extends IDisposable {
	readonly element: HTMLElement;

	activate?(): void;
	cancelPendingChanges?(): void;
	setNavigationTarget?(targetId: string | undefined): SettingsPaneNavigationTarget | undefined;
	setQuery?(query: string): void;
}

export interface SettingsPaneFactory {
	create(container: HTMLElement): SettingsPane;
}

/** Resolves one Settings pane factory for each product-owned section ID. */
export class SettingsPaneRegistry {
	private readonly factories = new Map<string, SettingsPaneFactory>();

	register(sectionId: string, factory: SettingsPaneFactory): void {
		if (!sectionId || sectionId !== sectionId.trim()) {
			throw new TypeError('Settings pane section IDs must be non-empty and must not have surrounding whitespace');
		}
		if (this.factories.has(sectionId)) {
			throw new Error(`Settings pane is already registered: ${sectionId}`);
		}
		this.factories.set(sectionId, factory);
	}

	create(sectionId: string, container: HTMLElement): SettingsPane {
		const factory = this.factories.get(sectionId);
		if (!factory) throw new RangeError(`No Settings pane is registered for '${sectionId}'`);
		return factory.create(container);
	}

	has(sectionId: string): boolean {
		return this.factories.has(sectionId);
	}
}
