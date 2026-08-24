import { Emitter } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import { ConfigurationsRegistry, type ConfigurationRegistry, type IConfigurationSettingSchema } from '../../../../platform/configuration/common/configurationRegistry.js';
import type { IConfigurationKey, IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type { IBooleanSetting, INumberSetting, ISelectSetting, ISetting, ISettingsEditorModel, ISettingsGroup, ISettingsSection, ITextSetting, SettingReference, SettingsStatus, SettingValueBinding } from './preferences.js';

export interface SettingState<T> {
	readonly id: string;
	readonly value: T;
	readonly defaultValue: T;
	readonly isDefault: boolean;
	readonly isPending: boolean;
}

/** Resolves one addressable setting and emits only that setting's state changes. */
export class SettingModel<T> extends DisposableOwner implements SettingReference {
	private readonly changeEmitter = this.own(new Emitter<SettingState<T>>());
	private value: T;
	private pending = false;

	public readonly onDidChange = this.changeEmitter.event;

	constructor(private readonly binding: SettingValueBinding<T>) {
		super();
		this.value = binding.getValue();
		if (binding.onDidChange) this.own(binding.onDidChange(() => this.refresh()));
	}

	public get id(): string {
		return this.binding.id;
	}

	public get state(): SettingState<T> {
		return {
			id: this.binding.id,
			value: this.value,
			defaultValue: this.binding.defaultValue,
			isDefault: Object.is(this.value, this.binding.defaultValue),
			isPending: this.pending,
		};
	}

	public isDefault(): boolean {
		return Object.is(this.value, this.binding.defaultValue);
	}

	public async update(value: T): Promise<void> {
		if (this.pending) return;
		this.value = value;
		this.setPending(true);
		try {
			await this.binding.updateValue(value);
		} finally {
			this.value = this.binding.getValue();
			this.setPending(false);
		}
	}

	public async reset(): Promise<void> {
		if (this.pending) return;
		this.value = this.binding.defaultValue;
		this.setPending(true);
		try {
			await this.binding.resetValue();
		} finally {
			this.value = this.binding.getValue();
			this.setPending(false);
		}
	}

	public refresh(): void {
		const value = this.binding.getValue();
		if (Object.is(value, this.value)) return;
		this.value = value;
		this.changeEmitter.fire(this.state);
	}

	private setPending(pending: boolean): void {
		if (pending === this.pending) return;
		this.pending = pending;
		this.changeEmitter.fire(this.state);
	}
}

export function configurationSettingBinding<T>(configurationService: IConfigurationService, key: IConfigurationKey<T>): SettingValueBinding<T> {
	return {
		id: key.key,
		defaultValue: key.defaultValue,
		onDidChange: listener => configurationService.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration(key)) listener();
		}),
		getValue: () => configurationService.getValue(key),
		updateValue: value => configurationService.updateValue(key, value),
		resetValue: () => configurationService.resetValue(key),
	};
}

/** Projects registered configuration schemas into Settings-domain entries. */
export class DefaultSettings {
	private readonly settings = new Map<string, ISetting>();

	constructor(registry: ConfigurationRegistry = ConfigurationsRegistry) {
		for (const configuration of registry.getRegisteredConfigurations()) {
			if (!configuration.setting) continue;
			this.settings.set(configuration.key.key, registeredSetting(configuration.key, configuration.setting));
		}
	}

	public get all(): readonly ISetting[] {
		return [...this.settings.values()];
	}

	public get<T>(key: IConfigurationKey<T>): ISetting {
		const setting = this.settings.get(key.key);
		if (!setting) throw new RangeError(`Configuration '${key.key}' does not declare Settings metadata`);
		return setting;
	}
}

function registeredSetting(key: IConfigurationKey<unknown>, schema: IConfigurationSettingSchema): ISetting {
	const base = {
		id: key.key,
		title: schema.title,
		description: schema.description,
		keywords: schema.keywords,
	};
	switch (schema.valueType) {
		case 'boolean':
			return { ...base, valueType: 'boolean', key: key as IConfigurationKey<boolean> };
		case 'number':
			return { ...base, valueType: 'number', key: key as IConfigurationKey<number>, minimum: schema.minimum, maximum: schema.maximum };
		case 'select':
			return {
				...base,
				valueType: 'select',
				key: key as IConfigurationKey<string>,
				get options() { return schema.options; },
			};
		case 'text':
			return { ...base, valueType: 'text', key: key as IConfigurationKey<string>, placeholder: schema.placeholder };
	}
}

/** Owns the immutable Settings projection and transient editor status. */
export class SettingsEditorModel extends DisposableOwner implements ISettingsEditorModel {
	private readonly sections = new Map<string, ISettingsSection>();
	private readonly statusEmitter = this.own(new Emitter<SettingsStatus>());

	public readonly onDidChangeStatus = this.statusEmitter.event;

	public readonly reportStatus = (message: string, isError: boolean): void => {
		this.statusEmitter.fire({ message, isError });
	};

	public get sectionIds(): readonly string[] {
		return [...this.sections.keys()];
	}

	constructor(sections: readonly ISettingsSection[]) {
		super();
		for (const section of sections) {
			if (!section.sectionId) throw new TypeError('Settings section IDs must not be empty');
			if (this.sections.has(section.sectionId)) throw new Error(`Settings section is already registered: ${section.sectionId}`);
			this.sections.set(section.sectionId, section);
		}
	}

	public hasSection(sectionId: string): boolean {
		return this.sections.has(sectionId);
	}

	public getSectionGroups(sectionId: string): readonly ISettingsGroup[] {
		return this.sections.get(sectionId)?.groups ?? [];
	}
}
