import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IClipboardService } from "../../../../platform/clipboard/common/clipboardService.js";
import type { ILanguagePackService, LanguagePackPackage } from "../../../../platform/languagePacks/common/languagePacksService.js";
import { LocalizationConfiguration, type ILocaleService } from "../../../services/localization/common/locale.js";
import type { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import { setSettingsItemIdentity } from "../../preferences/browser/settingsItem.js";
import { SettingsItemActions } from "../../preferences/browser/settingsItemActions.js";

/** Workbench contribution for selecting and installing client-local display languages. */
export class LocalizationSettingsPane extends DisposableOwner {
	readonly element: HTMLElement;
	private readonly document: Document;
	private readonly localization: ILocalizationService;
	private readonly localeService: ILocaleService;
	private readonly languagePacks: ILanguagePackService;
	private readonly status: HTMLParagraphElement;
	private languagePackPackages: readonly LanguagePackPackage[] = [];
	private isLoadingPackages = false;
	private languagePackError = "";
	private readonly rendered = this.own(new ResettableDisposableGroup());

	constructor(
		container: HTMLElement,
		localization: ILocalizationService,
		localeService: ILocaleService,
		languagePacks: ILanguagePackService,
		private readonly contextMenuProvider: IContextMenuProvider,
		private readonly clipboardService: IClipboardService,
	) {
		super();
		this.localization = localization;
		this.localeService = localeService;
		this.languagePacks = languagePacks;
		this.document = container.ownerDocument;
		this.element = h(this.document, "section");
		this.element.className = "zeta-localization-settings";
		this.status = h(this.document, "p");
		this.status.className = "zeta-settings-message";
		container.append(this.element);
		this.own(localization.onDidChange(() => this.render()));
		this.own(languagePacks.onDidChange(() => { void this.loadLanguagePackPackages(); }));
		void Promise.all([localization.whenReady, localeService.whenReady]).then(() => this.render());
		void this.loadLanguagePackPackages();
		this.render();
	}

	private render(): void {
		this.rendered.clear();
		const title = h(this.document, "h3");
		title.textContent = this.localization.translate("zeta.settings", "displayLanguage.title", "Display Language");
		const description = h(this.document, "p");
		description.textContent = this.localization.translate("zeta.settings", "displayLanguage.description", "Choose the language used by the Zeta interface.");
		const setting = h(this.document, "div");
		setting.className = "zeta-localization-setting";
		const label = h(this.document, "label");
		label.textContent = this.localization.translate("zeta.settings", "displayLanguage.select", "Interface language");
		const select = h(this.document, "select");
		select.className = "zeta-settings-select";
		for (const locale of this.languagePacks.availableLocales) {
			const option = h(this.document, "option");
			option.value = locale.locale;
			option.textContent = locale.localizedLanguageName + " (" + locale.languageName + ") · " + (locale.source === "builtin" ? this.localization.translate("zeta.settings", "displayLanguage.builtin", "Built-in") : this.localization.translate("zeta.settings", "displayLanguage.marketplace", "Marketplace"));
			option.selected = locale.locale === this.localeService.locale;
			select.append(option);
		}
		this.rendered.add(addDisposableListener(select, "change", () => {
			void this.localeService.setLocale(select.value).catch((error: unknown) => {
				this.status.textContent = error instanceof Error ? error.message : "Unable to change display language.";
			});
		}));
		const note = h(this.document, "p");
		note.textContent = this.localization.translate("zeta.settings", "displayLanguage.restart", "Some interface areas update after reopening Settings.");
		this.status.textContent = this.isLoadingPackages
			? this.localization.translate("zeta.settings", "displayLanguage.loading", "Loading available languages…")
			: this.languagePackError;
		setting.append(label, select, note, this.status);
		this.rendered.add(new SettingsItemActions(setting, {
			label: this.localization.translate("zeta.settings", "displayLanguage.select", "Interface language"),
			reference: {
				id: LocalizationConfiguration.locale.key,
				isDefault: () => this.localeService.locale === LocalizationConfiguration.locale.defaultValue,
				reset: () => this.localeService.setLocale(LocalizationConfiguration.locale.defaultValue),
			},
			contextMenuProvider: this.contextMenuProvider,
			clipboardService: this.clipboardService,
			onError: error => {
				this.status.textContent = error instanceof Error ? error.message : "Unable to run the setting action.";
			},
		}));
		this.element.replaceChildren(title, description, setting);
		if (this.languagePackPackages.length > 0 || this.isLoadingPackages) this.renderLanguagePackPackages();
	}

	private renderLanguagePackPackages(): void {
		const heading = h(this.document, "h4");
		heading.textContent = this.localization.translate("zeta.settings", "displayLanguage.installMore", "Install more languages from Marketplace");
		const list = h(this.document, "ul");
		for (const packageValue of this.languagePackPackages) {
			const item = h(this.document, "li");
			item.className = "zeta-localization-package";
			setSettingsItemIdentity(item, `languagePacks.${packageValue.id}@${packageValue.version}`, "resource");
			const label = h(this.document, "span");
			label.textContent = packageValue.displayName + " · " + packageValue.version;
			const installed = this.languagePacks.installedPackages.some(candidate => candidate.id === packageValue.id && candidate.version === packageValue.version);
			item.append(label);
			const action = this.rendered.add(new Button(item, {
				label: installed
					? this.localization.translate("zeta.settings", "displayLanguage.installed", "Installed")
					: this.localization.translate("zeta.settings", "displayLanguage.install", "Install"),
				presentation: "primary",
				enabled: !this.isLoadingPackages && !installed,
				onClick: () => {
					action.enabled = false;
					void this.languagePacks.install(packageValue.id, packageValue.version)
						.then(() => this.loadLanguagePackPackages())
						.catch((error: unknown) => {
							this.status.textContent = error instanceof Error ? error.message : "Unable to install language pack.";
							action.enabled = true;
						});
				},
			}));
			list.append(item);
		}
		this.element.append(heading, list);
	}

	private async loadLanguagePackPackages(): Promise<void> {
		this.isLoadingPackages = true;
		this.render();
		try {
			this.languagePackPackages = await this.languagePacks.search("", 100);
			this.languagePackError = "";
		} catch {
			this.languagePackError = this.localization.translate("zeta.settings", "displayLanguage.unavailable", "Marketplace language packs are unavailable.");
		} finally {
			this.isLoadingPackages = false;
			this.render();
		}
	}
}
