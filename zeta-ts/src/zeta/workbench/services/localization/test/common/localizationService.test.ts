import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationService } from "../../../../../platform/configuration/common/configurationService.js";
import type { IMarketplaceService } from "../../../../../platform/marketplace/common/marketplaceService.js";
import { MarketplaceLanguagePackService } from "../../../../../platform/languagePacks/browser/marketplaceLanguagePackService.js";
import { builtinLanguagePackCatalogs } from "../../common/localizationCatalogs.js";
import { LocalizationConfiguration, WorkbenchLocaleService, normalizeLocale, resolveLocale } from "../../common/locale.js";
import { WorkbenchLocalizationService } from "../../browser/workbenchLocalizationService.js";

test("locale resolution prefers exact, base-language, and English fallback matches", () => {
	assert.equal(normalizeLocale("ZH_cn"), "zh-CN");
	assert.equal(resolveLocale("zh-cn", ["en", "zh-CN", "fr"]), "zh-CN");
	assert.equal(resolveLocale("fr-CA", ["en", "fr"]), "fr");
	assert.equal(resolveLocale("de", ["en", "zh-CN"]), "en");
});

test("locale selection is client-local and only accepts installed packs", async () => {
	using configuration = new TestConfigurationService();
	using languagePacks = new MarketplaceLanguagePackService(createMarketplace(), builtinLanguagePackCatalogs);
	using localeService = new WorkbenchLocaleService(configuration, languagePacks);

	await localeService.whenReady;
	await assert.rejects(localeService.setLocale("fr"), /not installed/);
	await localeService.setLocale("zh_cn");
	assert.equal(localeService.locale, "zh-CN");
	assert.equal(configuration.getValue(LocalizationConfiguration.locale), "zh-CN");
});

test("localization lookup falls back to English and formats parameters", async () => {
	using configuration = new TestConfigurationService();
	const languagePacks = new MarketplaceLanguagePackService(createMarketplace(), builtinLanguagePackCatalogs);
	using localeService = new WorkbenchLocaleService(configuration, languagePacks);
	using localization = new WorkbenchLocalizationService(localeService, languagePacks);

	await localization.whenReady;
	assert.equal(localization.translate("zeta.settings", "displayLanguage.title", "Fallback"), "Display Language");
	assert.equal(localization.translate("zeta.missing", "missing", "Hello {name}", { name: "Ada" }), "Hello Ada");
});

function createMarketplace(): IMarketplaceService {
	const changes = new Emitter<void>();
	return {
		onDidChangeInstalled: changes.event,
		cachedBrowse: () => undefined,
		browse: () => Promise.reject(new Error("unused")),
		refreshBrowse: () => Promise.reject(new Error("unused")),
		search: async () => [],
		get: () => Promise.reject(new Error("unused")),
		download: () => Promise.reject(new Error("unused")),
		install: () => Promise.reject(new Error("unused")),
		update: () => Promise.reject(new Error("unused")),
		uninstall: () => Promise.reject(new Error("unused")),
		listInstalled: async () => [],
		acquireCapability: () => Promise.reject(new Error("unused")),
		releaseCapability: async () => {},
		openResource: () => Promise.reject(new Error("unused")),
	};
}

class TestConfigurationService implements IConfigurationService {
	private readonly changes = new Emitter<IConfigurationChangeEvent>();
	private readonly values = new Map<string, unknown>();

	readonly onDidChangeConfiguration = this.changes.event;

	getValue<T>(key: IConfigurationKey<T>): T {
		return (this.values.get(key.key) ?? key.defaultValue) as T;
	}

	async updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
		this.values.set(key.key, key.parse(key.serialize(value)));
		this.changes.fire({
			keys: new Set([key.key]),
			affectsConfiguration: candidate => candidate.key === key.key,
		});
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		this.values.delete(key.key);
		this.changes.fire({
			keys: new Set([key.key]),
			affectsConfiguration: candidate => candidate.key === key.key,
		});
	}

	async reload(): Promise<void> {}

	[Symbol.dispose](): void {
		this.changes.dispose();
	}
}
