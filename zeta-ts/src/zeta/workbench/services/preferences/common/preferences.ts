import type { Event } from '../../../../base/common/event.js';
import type { IDisposable } from '../../../../base/common/lifecycle.js';
import type { IConfigurationKey } from '../../../../platform/configuration/common/configurationService.js';
import { createServiceIdentifier } from '../../../../platform/instantiation/common/instantiation.js';

export type SettingValueType = 'boolean' | 'number' | 'select' | 'text';
export type SettingsPresentation = 'editor' | 'general';

export interface SettingValueBinding<T> {
	readonly id: string;
	readonly defaultValue: T;
	readonly onDidChange?: Event<void>;

	getValue(): T;
	updateValue(value: T): Promise<void>;
	resetValue(): Promise<void>;
}

export interface SettingReference {
	readonly id: string;

	isDefault(): boolean;
	reset(): Promise<void>;
}

export interface ISettingBase {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly keywords?: readonly string[];
	readonly presentation?: SettingsPresentation;
}

export interface IBooleanSetting extends ISettingBase {
	readonly valueType: 'boolean';
	readonly key: IConfigurationKey<boolean>;
}

export interface INumberSetting extends ISettingBase {
	readonly valueType: 'number';
	readonly key: IConfigurationKey<number>;
	readonly minimum: number;
	readonly maximum: number;
}

export interface ISelectSetting extends ISettingBase {
	readonly valueType: 'select';
	readonly key: IConfigurationKey<string>;
	readonly options: readonly ISelectSettingOption[];
}

export interface ITextSetting extends ISettingBase {
	readonly valueType: 'text';
	readonly key: IConfigurationKey<string>;
	readonly placeholder: string;
}

export type ISetting = IBooleanSetting | INumberSetting | ISelectSetting | ITextSetting;

export interface ISelectSettingOption {
	readonly value: string;
	readonly label: string;
}

export interface ISettingsGroup {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly settings: readonly ISetting[];
}

export interface ISettingsSection {
	readonly sectionId: string;
	readonly groups: readonly ISettingsGroup[];
}

export interface SettingsStatus {
	readonly message: string;
	readonly isError: boolean;
}

export interface ISettingsEditorModel extends IDisposable {
	readonly onDidChangeStatus: Event<SettingsStatus>;
	readonly sectionIds: readonly string[];
	readonly reportStatus: (message: string, isError: boolean) => void;

	hasSection(sectionId: string): boolean;
	getSectionGroups(sectionId: string): readonly ISettingsGroup[];
}

/** Workbench-level entry point for opening Preferences surfaces. */
export interface IPreferencesService {
	readonly onDidChangeSettingsVisibility: Event<boolean>;
	readonly onDidChangeSettingsSection: Event<string>;
	readonly isSettingsOpen: boolean;
	readonly activeSettingsSectionId: string;

	openSettings(sectionId?: string): void;
	closeSettings(): void;
	openKeybindings(): Promise<void>;
}

export const IPreferencesService = createServiceIdentifier<IPreferencesService>('preferencesService');
