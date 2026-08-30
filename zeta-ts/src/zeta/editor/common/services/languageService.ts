import { Emitter, Event, type Event as BaseEvent } from '../../../base/common/event.js';
import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { type URI } from '../../../base/common/uri.js';
import type { TextResourceLanguageInput } from '../../../platform/language/common/textResourceLanguage.js';
import { LanguageRegistry, type LanguageDescription, type LanguageDescriptionContribution, type LanguageDescriptionRegistration, type LanguageRegistrationOptions } from '../languages/languageRegistry.js';
import { type ILanguageIdCodec } from '../languages.js';
import { LanguageId } from '../encodedTokenAttributes.js';
import { type ILanguageIcon, type ILanguageNameIdPair, type ILanguageSelection, type IZetaLanguageService } from '../languages/language.js';

/** Owns language identities and file associations independently of feature providers. */
export class LanguageService extends Disposable implements IZetaLanguageService {
	readonly _serviceBrand = undefined;
	public readonly languages = this._register(new LanguageRegistry());
	private readonly basicFeaturesEmitter = this._register(new Emitter<string>());
	private readonly richFeaturesEmitter = this._register(new Emitter<string>());
	private readonly requestedBasic = new Set<string>();
	private readonly requestedRich = new Set<string>();
	readonly languageIdCodec: ILanguageIdCodec = new LanguageIdCodec();
	readonly onDidRequestBasicLanguageFeatures = this.basicFeaturesEmitter.event;
	readonly onDidRequestRichLanguageFeatures = this.richFeaturesEmitter.event;
	readonly onDidChange: BaseEvent<void> = listener => this.languages.onDidChange(() => listener());

	public registerLanguage(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
		return this.languages.register(description, options);
	}

	public registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration {
		return this.languages.registerMany(contributions);
	}

	public resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
		return this.languages.resolveLanguageId(input);
	}

	isRegisteredLanguageId(languageId: string): boolean { return this.languages.get(languageId) !== undefined; }
	getRegisteredLanguageIds(): string[] { return this.languages.getRegisteredLanguageIds(); }
	getSortedRegisteredLanguageNames(): ILanguageNameIdPair[] { return this.languages.getSortedRegisteredLanguageNames(); }
	getLanguageName(languageId: string): string | null { return this.languages.getLanguageName(languageId); }
	getMimeType(languageId: string): string | null { return this.languages.get(languageId)?.mimetypes?.[0] ?? null; }
	getIcon(languageId: string): ILanguageIcon | null { return this.languages.get(languageId)?.icon ?? null; }
	getExtensions(languageId: string): ReadonlyArray<string> { return this.languages.get(languageId)?.extensions ?? []; }
	getFilenames(languageId: string): ReadonlyArray<string> { return this.languages.get(languageId)?.filenames ?? []; }
	getConfigurationFiles(languageId: string): ReadonlyArray<URI> {
		const configuration = this.languages.get(languageId)?.configuration;
		return configuration ? [configuration] : [];
	}
	getLanguageIdByLanguageName(languageName: string): string | null { return this.languages.getLanguageIdByLanguageName(languageName); }
	getLanguageIdByMimeType(mimeType: string | null | undefined): string | null { return this.languages.getLanguageIdByMimeType(mimeType); }
	guessLanguageIdByFilepathOrFirstLine(resource: URI, firstLine?: string): string | null { return this.resolveLanguageId({ resource, firstLine }) ?? null; }
	createById(languageId: string | null | undefined): ILanguageSelection { return fixedSelection(languageId && this.isRegisteredLanguageId(languageId) ? languageId : 'plaintext'); }
	createByMimeType(mimeType: string | null | undefined): ILanguageSelection { return this.createById(this.getLanguageIdByMimeType(mimeType)); }
	createByFilepathOrFirstLine(resource: URI | null, firstLine?: string): ILanguageSelection { return this.createById(resource ? this.guessLanguageIdByFilepathOrFirstLine(resource, firstLine) : null); }

	requestBasicLanguageFeatures(languageId: string): void {
		if (this.requestedBasic.has(languageId)) return;
		this.requestedBasic.add(languageId);
		this.basicFeaturesEmitter.fire(languageId);
	}

	requestRichLanguageFeatures(languageId: string): void {
		this.requestBasicLanguageFeatures(languageId);
		if (this.requestedRich.has(languageId)) return;
		this.requestedRich.add(languageId);
		this.richFeaturesEmitter.fire(languageId);
	}
}

class LanguageIdCodec implements ILanguageIdCodec {
	private readonly ids = new Map<string, LanguageId>([['plaintext', LanguageId.PlainText]]);
	private readonly languages = new Map<LanguageId, string>([[LanguageId.PlainText, 'plaintext']]);

	encodeLanguageId(languageId: string): LanguageId {
		const existing = this.ids.get(languageId);
		if (existing !== undefined) return existing;
		const id = this.ids.size + 1;
		this.ids.set(languageId, id);
		this.languages.set(id, languageId);
		return id;
	}

	decodeLanguageId(languageId: LanguageId): string { return this.languages.get(languageId) ?? 'plaintext'; }
}

function fixedSelection(languageId: string): ILanguageSelection {
	return Object.freeze({ languageId, onDidChange: Event.None });
}
