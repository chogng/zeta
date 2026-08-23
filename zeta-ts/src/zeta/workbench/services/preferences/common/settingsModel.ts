import { Emitter, type Event } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { IConfigurationKey, IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';

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

export interface SettingsItemState<T> {
	readonly id: string;
	readonly value: T;
	readonly defaultValue: T;
	readonly isDefault: boolean;
	readonly isPending: boolean;
}

/** Resolves one addressable Settings item and emits only that item's state changes. */
export class SettingsItemModel<T> extends DisposableOwner implements SettingReference {
	private readonly changeEmitter = this.own(new Emitter<SettingsItemState<T>>());
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

	public get state(): SettingsItemState<T> {
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
		this.setPending(true);
		try {
			await this.binding.updateValue(value);
			this.refresh();
		} finally {
			this.setPending(false);
			this.refresh();
		}
	}

	public async reset(): Promise<void> {
		if (this.pending) return;
		this.setPending(true);
		try {
			await this.binding.resetValue();
			this.refresh();
		} finally {
			this.setPending(false);
			this.refresh();
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
