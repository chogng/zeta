import type { IConfigurationKey } from '../../../../platform/configuration/common/configurationService.js';
import type { SettingValueBinding } from '../../../services/preferences/common/settingsModel.js';

interface SettingDescriptorBase {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly keywords?: readonly string[];
}

export interface ActionSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'action';
	readonly actionLabel: string;
	run(): void | Promise<void>;
}

export interface BooleanSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'boolean';
	readonly key: IConfigurationKey<boolean>;
}

export interface NumberSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'number';
	readonly key: IConfigurationKey<number>;
	readonly minimum: number;
	readonly maximum: number;
}

export interface SelectSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'select';
	readonly key: IConfigurationKey<string>;
	readonly options: readonly SelectSettingOption[];
}

export interface BoundSelectSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'boundSelect';
	readonly binding: SettingValueBinding<string>;
	readonly options: readonly SelectSettingOption[];
}

export interface TextSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'text';
	readonly key: IConfigurationKey<string>;
	readonly placeholder: string;
}

export interface InformationSettingDescriptor extends SettingDescriptorBase {
	readonly kind: 'information';
	readonly stateLabel: string;
}

export type ConfigurationSettingDescriptor = ActionSettingDescriptor | BooleanSettingDescriptor | BoundSelectSettingDescriptor | InformationSettingDescriptor | NumberSettingDescriptor | SelectSettingDescriptor | TextSettingDescriptor;

export interface ConfigurationSettingsGroupDescriptor {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly settings: readonly ConfigurationSettingDescriptor[];
}

export interface SelectSettingOption {
	readonly value: string;
	readonly label: string;
}

export function actionSetting(id: string, title: string, description: string, actionLabel: string, run: () => void | Promise<void>): ActionSettingDescriptor {
	return { kind: 'action', id, title, description, actionLabel, run };
}

export function booleanSetting(key: IConfigurationKey<boolean>, title: string, description: string): BooleanSettingDescriptor {
	return { kind: 'boolean', id: key.key, key, title, description };
}

export function informationSetting(id: string, title: string, description: string, stateLabel: string): InformationSettingDescriptor {
	return { kind: 'information', id, title, description, stateLabel };
}

export function numberSetting(key: IConfigurationKey<number>, title: string, description: string, minimum: number, maximum: number): NumberSettingDescriptor {
	return { kind: 'number', id: key.key, key, title, description, minimum, maximum };
}

export function selectSetting<T extends string>(key: IConfigurationKey<T>, title: string, description: string, options: readonly { readonly value: T; readonly label: string }[]): SelectSettingDescriptor {
	return { kind: 'select', id: key.key, key: key as unknown as IConfigurationKey<string>, title, description, options };
}

export function boundSelectSetting<T extends string>(binding: SettingValueBinding<T>, title: string, description: string, options: readonly { readonly value: T; readonly label: string }[]): BoundSelectSettingDescriptor {
	return {
		kind: 'boundSelect',
		id: binding.id,
		binding: binding as unknown as SettingValueBinding<string>,
		title,
		description,
		options,
	};
}

export function textSetting(key: IConfigurationKey<string>, title: string, description: string, placeholder: string): TextSettingDescriptor {
	return { kind: 'text', id: key.key, key, title, description, placeholder };
}
