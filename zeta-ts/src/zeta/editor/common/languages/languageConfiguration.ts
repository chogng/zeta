import { Emitter, type Event } from "../../../base/common/event.js";
import { Disposable, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertLanguageId } from "./languageId.js";

export interface LanguageCharacterPair {
	readonly open: string;
	readonly close: string;
}

export type LanguageAutoClosingTokenContext = "string" | "comment";

export interface LanguageAutoClosingPair extends LanguageCharacterPair {
	readonly notIn?: readonly LanguageAutoClosingTokenContext[];
}

export interface LanguageCommentConfiguration {
	readonly lineComment?: string | null;
	readonly blockComment?: LanguageCharacterPair | null;
}

export enum LanguageIndentAction {
	None = "none",
	Indent = "indent",
	IndentOutdent = "indentOutdent",
	Outdent = "outdent",
}

export interface LanguageEnterAction {
	readonly indentAction: LanguageIndentAction;
	readonly appendText?: string;
	readonly removeText?: number;
}

export interface LanguageOnEnterRule {
	readonly beforeText: RegExp;
	readonly afterText?: RegExp;
	readonly previousLineText?: RegExp;
	readonly action: LanguageEnterAction;
}

export interface LanguageIndentationRules {
	readonly decreaseIndentPattern: RegExp;
	readonly increaseIndentPattern: RegExp;
	readonly indentNextLinePattern?: RegExp | null;
	readonly unIndentedLinePattern?: RegExp | null;
}

/**
 * Language-owned markers for named fold regions such as `// #region`.
 *
 * Both patterns are matched against the complete physical line. Contributions
 * should include their comment delimiter so ordinary source text cannot create
 * a fold by merely containing a marker name.
 */
export interface LanguageFoldingMarkers {
	readonly start: RegExp;
	readonly end: RegExp;
}

/** DOM-free editing rules contributed for one language. */
export interface LanguageConfiguration {
	readonly comments?: LanguageCommentConfiguration | null;
	readonly brackets?: readonly LanguageCharacterPair[] | null;
	readonly autoClosingPairs?: readonly LanguageAutoClosingPair[] | null;
	readonly surroundingPairs?: readonly LanguageCharacterPair[] | null;
	readonly autoCloseBefore?: string | null;
	readonly indentationRules?: LanguageIndentationRules | null;
	readonly foldingMarkers?: LanguageFoldingMarkers | null;
	readonly onEnterRules?: readonly LanguageOnEnterRule[] | null;
	/** Optional language-specific word matcher for editor selection gestures. */
	readonly wordPattern?: RegExp | null;
}

export interface ResolvedLanguageCommentConfiguration {
	readonly lineComment?: string;
	readonly blockComment?: LanguageCharacterPair;
}

/** Immutable field-wise composition of all current language contributions. */
export interface ResolvedLanguageConfiguration {
	readonly languageId: string;
	readonly revision: number;
	readonly comments: ResolvedLanguageCommentConfiguration;
	readonly brackets: readonly LanguageCharacterPair[];
	readonly autoClosingPairs: readonly LanguageAutoClosingPair[];
	readonly surroundingPairs: readonly LanguageCharacterPair[];
	readonly autoCloseBefore: string;
	readonly indentationRules?: LanguageIndentationRules;
	readonly foldingMarkers?: LanguageFoldingMarkers;
	readonly onEnterRules: readonly LanguageOnEnterRule[];
	readonly wordPattern?: RegExp;
}

export interface LanguageConfigurationRegistrationOptions {
	readonly priority?: number;
}

export interface LanguageConfigurationContributionInput {
	readonly languageId: string;
	readonly configuration: LanguageConfiguration;
	readonly options?: LanguageConfigurationRegistrationOptions;
}

/** One caller-owned configuration set that can be atomically replaced. */
export interface LanguageConfigurationRegistration extends IDisposable {
	replace(contributions: readonly LanguageConfigurationContributionInput[]): void;
}

export interface LanguageConfigurationChangeEvent {
	readonly languageId: string;
	readonly configuration: ResolvedLanguageConfiguration;
}

export interface LanguageConfigurationSource {
	readonly onDidChangeConfiguration?: Event<LanguageConfigurationChangeEvent>;
	getLanguageConfiguration(languageId: string): ResolvedLanguageConfiguration;
}

interface LanguageConfigurationContribution {
	readonly owner: object;
	readonly configuration: NormalizedLanguageConfiguration;
	readonly priority: number;
	readonly order: number;
}

interface NormalizedLanguageCommentConfiguration {
	readonly lineComment?: string | null;
	readonly blockComment?: LanguageCharacterPair | null;
}

interface NormalizedLanguageConfiguration {
	readonly comments?: NormalizedLanguageCommentConfiguration | null;
	readonly brackets?: readonly LanguageCharacterPair[] | null;
	readonly autoClosingPairs?: readonly LanguageAutoClosingPair[] | null;
	readonly surroundingPairs?: readonly LanguageCharacterPair[] | null;
	readonly autoCloseBefore?: string | null;
	readonly indentationRules?: LanguageIndentationRules | null;
	readonly foldingMarkers?: LanguageFoldingMarkers | null;
	readonly onEnterRules?: readonly LanguageOnEnterRule[] | null;
	readonly wordPattern?: RegExp | null;
}

export const DEFAULT_LANGUAGE_AUTO_CLOSE_BEFORE = "\"'`;:.,=}])> \n\t";

/** Caller-owned registry for composable language editing rules. */
export class LanguageConfigurationRegistry extends Disposable implements LanguageConfigurationSource {
	private readonly changeEmitter = this._register(new Emitter<LanguageConfigurationChangeEvent>());
	private readonly contributions = new Map<string, LanguageConfigurationContribution[]>();
	private readonly revisions = new Map<string, number>();
	private readonly resolved = new Map<string, ResolvedLanguageConfiguration>();
	private nextOrder = 1;

	readonly onDidChangeConfiguration: Event<LanguageConfigurationChangeEvent> = this.changeEmitter.event;

	constructor() {
		super();
		this._register(toDisposable(() => {
			this.contributions.clear();
			this.revisions.clear();
			this.resolved.clear();
		}));
	}

	register(languageId: string, configuration: LanguageConfiguration, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
		return this.registerMany([{ languageId, configuration, options }]);
	}

	registerMany(contributions: readonly LanguageConfigurationContributionInput[]): LanguageConfigurationRegistration {
		this.assertNotDisposed();
		const owner = Object.freeze({});
		this.replace(owner, contributions);
		let disposed = false;
		const registration = toDisposable(() => {
			if (disposed) return;
			disposed = true;
			this.removeOwner(owner);
		}) as LanguageConfigurationRegistration;
		registration.replace = replacement => {
			if (disposed) throw new ReferenceError("Language configuration registration is already disposed");
			this.assertNotDisposed();
			this.replace(owner, replacement);
		};
		return registration;
	}

	getLanguageConfiguration(languageId: string): ResolvedLanguageConfiguration {
		this.assertNotDisposed();
		assertLanguageId(languageId);
		const cached = this.resolved.get(languageId);
		if (cached) return cached;
		const configuration = resolveLanguageConfiguration(languageId, this.revisions.get(languageId) ?? 0, this.contributions.get(languageId) ?? []);
		this.resolved.set(languageId, configuration);
		return configuration;
	}

	private publishChange(languageId: string): void {
		this.revisions.set(languageId, (this.revisions.get(languageId) ?? 0) + 1);
		this.resolved.delete(languageId);
		this.changeEmitter.fire(Object.freeze({
			languageId,
			configuration: this.getLanguageConfiguration(languageId),
		}));
	}


	private replace(owner: object, contributions: readonly LanguageConfigurationContributionInput[]): void {
		if (!Array.isArray(contributions)) throw new TypeError("Language configuration contributions must be an array");
		const entries = contributions.map(input => {
			if (typeof input !== "object" || input === null) throw new TypeError("Language configuration contribution must be an object");
			assertLanguageId(input.languageId);
			return Object.freeze({ languageId: input.languageId, contribution: Object.freeze({ owner, configuration: normalizeLanguageConfiguration(input.configuration), priority: normalizePriority(input.options ?? {}), order: this.nextOrder++ }) });
		});
		const affected = this.ownerLanguageIds(owner);
		for (const entry of entries) affected.add(entry.languageId);
		this.deleteOwner(owner);
		for (const entry of entries) {
			const values = this.contributions.get(entry.languageId);
			if (values) values.push(entry.contribution);
			else this.contributions.set(entry.languageId, [entry.contribution]);
		}
		for (const languageId of [...affected].sort()) this.publishChange(languageId);
	}

	private removeOwner(owner: object): void {
		const affected = this.ownerLanguageIds(owner);
		this.deleteOwner(owner);
		if (!this.isDisposed) for (const languageId of [...affected].sort()) this.publishChange(languageId);
	}

	private deleteOwner(owner: object): void {
		for (const [languageId, entries] of this.contributions) {
			const remaining = entries.filter(entry => entry.owner !== owner);
			if (remaining.length === 0) this.contributions.delete(languageId);
			else if (remaining.length !== entries.length) this.contributions.set(languageId, remaining);
		}
	}

	private ownerLanguageIds(owner: object): Set<string> {
		const languageIds = new Set<string>();
		for (const [languageId, entries] of this.contributions) if (entries.some(entry => entry.owner === owner)) languageIds.add(languageId);
		return languageIds;
	}
}

function normalizeLanguageConfiguration(configuration: LanguageConfiguration): NormalizedLanguageConfiguration {
	if (typeof configuration !== "object" || configuration === null) {
		throw new TypeError("Language configuration must be an object");
	}
	const comments = normalizeComments(configuration.comments);
	const brackets = normalizeBrackets(configuration.brackets);
	const autoClosingPairs = normalizeAutoClosingPairs(configuration.autoClosingPairs);
	const surroundingPairs = normalizePairs(configuration.surroundingPairs, "Language surrounding");
	const autoCloseBefore = normalizeAutoCloseBefore(configuration.autoCloseBefore);
	const indentationRules = normalizeIndentationRules(configuration.indentationRules);
	const foldingMarkers = normalizeFoldingMarkers(configuration.foldingMarkers);
	const onEnterRules = normalizeOnEnterRules(configuration.onEnterRules);
	const wordPattern = configuration.wordPattern === undefined ? undefined
		: configuration.wordPattern === null ? null : normalizePattern(configuration.wordPattern, "Language word pattern");
	return Object.freeze({
		...(comments === undefined ? {} : { comments }),
		...(brackets === undefined ? {} : { brackets }),
		...(autoClosingPairs === undefined ? {} : { autoClosingPairs }),
		...(surroundingPairs === undefined ? {} : { surroundingPairs }),
		...(autoCloseBefore === undefined ? {} : { autoCloseBefore }),
		...(indentationRules === undefined ? {} : { indentationRules }),
		...(foldingMarkers === undefined ? {} : { foldingMarkers }),
		...(onEnterRules === undefined ? {} : { onEnterRules }),
		...(wordPattern === undefined ? {} : { wordPattern }),
	});
}

function normalizeFoldingMarkers(markers: LanguageConfiguration["foldingMarkers"]): LanguageFoldingMarkers | null | undefined {
	if (markers === undefined || markers === null) return markers;
	if (typeof markers !== "object") throw new TypeError("Language folding markers must be an object");
	return Object.freeze({
		start: normalizePattern(markers.start, "Language folding start marker"),
		end: normalizePattern(markers.end, "Language folding end marker"),
	});
}

function normalizeIndentationRules(rules: LanguageConfiguration["indentationRules"]): LanguageIndentationRules | null | undefined {
	if (rules === undefined || rules === null) return rules;
	if (typeof rules !== "object") throw new TypeError("Language indentation rules must be an object");
	return Object.freeze({
		decreaseIndentPattern: normalizePattern(rules.decreaseIndentPattern, "Language decrease-indent pattern"),
		increaseIndentPattern: normalizePattern(rules.increaseIndentPattern, "Language increase-indent pattern"),
		...(rules.indentNextLinePattern === undefined ? {} : {
			indentNextLinePattern: rules.indentNextLinePattern === null ? null : normalizePattern(rules.indentNextLinePattern, "Language indent-next-line pattern"),
		}),
		...(rules.unIndentedLinePattern === undefined ? {} : {
			unIndentedLinePattern: rules.unIndentedLinePattern === null ? null : normalizePattern(rules.unIndentedLinePattern, "Language unindented-line pattern"),
		}),
	});
}

function normalizeOnEnterRules(rules: LanguageConfiguration["onEnterRules"]): readonly LanguageOnEnterRule[] | null | undefined {
	if (rules === undefined || rules === null) return rules;
	if (!Array.isArray(rules)) throw new TypeError("Language on-enter rules must be an array");
	return Object.freeze(rules.map(rule => {
		if (typeof rule !== "object" || rule === null) throw new TypeError("Language on-enter rule must be an object");
		return Object.freeze({
			beforeText: normalizePattern(rule.beforeText, "Language on-enter before-text pattern"),
			...(rule.afterText === undefined ? {} : { afterText: normalizePattern(rule.afterText, "Language on-enter after-text pattern") }),
			...(rule.previousLineText === undefined ? {} : { previousLineText: normalizePattern(rule.previousLineText, "Language on-enter previous-line pattern") }),
			action: normalizeEnterAction(rule.action),
		});
	}));
}

function normalizeEnterAction(action: LanguageEnterAction): LanguageEnterAction {
	if (typeof action !== "object" || action === null || !Object.values(LanguageIndentAction).includes(action.indentAction)) {
		throw new TypeError("Language on-enter action has an unknown indentation action");
	}
	if (action.appendText !== undefined && (typeof action.appendText !== "string" || /[\r\n]/.test(action.appendText))) {
		throw new TypeError("Language on-enter append text must be a single-line string");
	}
	if (action.removeText !== undefined && (!Number.isSafeInteger(action.removeText) || action.removeText < 0)) {
		throw new RangeError("Language on-enter remove text must be a non-negative safe integer");
	}
	return Object.freeze({
		indentAction: action.indentAction,
		...(action.appendText === undefined ? {} : { appendText: action.appendText }),
		...(action.removeText === undefined ? {} : { removeText: action.removeText }),
	});
}

function normalizePattern(pattern: RegExp, owner: string): RegExp {
	if (!(pattern instanceof RegExp)) throw new TypeError(`${owner} must be a RegExp`);
	return Object.freeze(new RegExp(pattern.source, pattern.flags));
}

function normalizeComments(comments: LanguageConfiguration["comments"]): NormalizedLanguageCommentConfiguration | null | undefined {
	if (comments === undefined || comments === null) return comments;
	if (typeof comments !== "object") throw new TypeError("Language comments configuration must be an object");
	const lineComment = normalizeOptionalToken(comments.lineComment, "Language line comment");
	const blockComment = comments.blockComment === undefined || comments.blockComment === null
		? comments.blockComment
		: normalizeMatchingPair(comments.blockComment, "Language block comment", true);
	return Object.freeze({
		...(lineComment === undefined ? {} : { lineComment }),
		...(blockComment === undefined ? {} : { blockComment }),
	});
}

function normalizeBrackets(brackets: LanguageConfiguration["brackets"]): readonly LanguageCharacterPair[] | null | undefined {
	if (brackets === undefined || brackets === null) return brackets;
	if (!Array.isArray(brackets)) throw new TypeError("Language brackets must be an array");
	const normalized = brackets.map(pair => normalizeMatchingPair(pair, "Language bracket"));
	const opens = new Set<string>();
	const closes = new Set<string>();
	for (const pair of normalized) {
		// Multiple opening forms may share a close token (for example `${` and `{` in
		// JavaScript); the lexical stack still resolves the matching close by nesting.
		if (opens.has(pair.open)) {
			throw new RangeError("Language bracket open tokens must be unique");
		}
		opens.add(pair.open);
		closes.add(pair.close);
	}
	if ([...opens].some(token => closes.has(token))) {
		throw new RangeError("Language bracket tokens must have unambiguous open and close roles");
	}
	return Object.freeze(normalized);
}

function normalizePairs(pairs: readonly LanguageCharacterPair[] | null | undefined, owner: string): readonly LanguageCharacterPair[] | null | undefined {
	if (pairs === undefined || pairs === null) return pairs;
	if (!Array.isArray(pairs)) throw new TypeError(`${owner} pairs must be an array`);
	const normalized = pairs.map(pair => normalizeCharacterPair(pair, owner, owner === "Language surrounding"));
	const opens = new Set<string>();
	for (const pair of normalized) {
		if (opens.has(pair.open)) throw new RangeError(`${owner} pair open tokens must be unique`);
		opens.add(pair.open);
	}
	return Object.freeze(normalized);
}

function normalizeAutoClosingPairs(pairs: readonly LanguageAutoClosingPair[] | null | undefined): readonly LanguageAutoClosingPair[] | null | undefined {
	const normalized = normalizePairs(pairs, "Language auto-closing");
	if (normalized === undefined || normalized === null) return normalized;
	return Object.freeze(normalized.map((pair, index) => {
		const notIn = pairs![index]!.notIn;
		if (notIn === undefined) return pair;
		if (!Array.isArray(notIn) || notIn.some(context => context !== "string" && context !== "comment")) {
			throw new TypeError("Language auto-closing notIn must contain only string or comment contexts");
		}
		if (new Set(notIn).size !== notIn.length) {
			throw new RangeError("Language auto-closing notIn contexts must be unique");
		}
		return Object.freeze({ ...pair, notIn: Object.freeze([...notIn]) });
	}));
}

function normalizeMatchingPair(pair: LanguageCharacterPair, owner: string, allowSameToken = false): LanguageCharacterPair {
	const normalized = normalizeCharacterPair(pair, owner);
	if (!allowSameToken && normalized.open === normalized.close) throw new RangeError(`${owner} open and close tokens must differ`);
	return normalized;
}

function normalizeCharacterPair(pair: LanguageCharacterPair, owner: string, allowEmptyClose = false): LanguageCharacterPair {
	if (typeof pair !== "object" || pair === null) throw new TypeError(`${owner} pair must be an object`);
	const open = normalizeToken(pair.open, `${owner} open token`);
	const close = allowEmptyClose && pair.close === "" ? "" : normalizeToken(pair.close, `${owner} close token`);
	return Object.freeze({ open, close });
}

function normalizeOptionalToken(value: string | null | undefined, owner: string): string | null | undefined {
	return value === undefined || value === null ? value : normalizeToken(value, owner);
}

function normalizeToken(value: unknown, owner: string): string {
	if (typeof value !== "string" || value.length === 0 || /[\r\n]/.test(value)) {
		throw new TypeError(`${owner} must be a non-empty single-line string`);
	}
	return value;
}

function normalizeAutoCloseBefore(value: string | null | undefined): string | null | undefined {
	if (value === undefined || value === null) return value;
	if (typeof value !== "string") throw new TypeError("Language auto-close-before value must be a string");
	return value;
}

function normalizePriority(options: LanguageConfigurationRegistrationOptions): number {
	if (typeof options !== "object" || options === null) {
		throw new TypeError("Language configuration registration options must be an object");
	}
	const priority = options.priority ?? 0;
	if (!Number.isSafeInteger(priority)) throw new RangeError("Language configuration priority must be a safe integer");
	return priority;
}

function resolveLanguageConfiguration(languageId: string, revision: number, contributions: readonly LanguageConfigurationContribution[]): ResolvedLanguageConfiguration {
	let lineComment: string | undefined;
	let blockComment: LanguageCharacterPair | undefined;
	let brackets: readonly LanguageCharacterPair[] = Object.freeze([]);
	let autoClosingPairs: readonly LanguageAutoClosingPair[] | undefined;
	let surroundingPairs: readonly LanguageCharacterPair[] | undefined;
	let autoCloseBefore: string | undefined;
	let indentationRules: LanguageIndentationRules | undefined;
	let foldingMarkers: LanguageFoldingMarkers | undefined;
	let onEnterRules: readonly LanguageOnEnterRule[] = Object.freeze([]);
	let wordPattern: RegExp | undefined;
	const ordered = [...contributions].sort((left, right) => left.priority - right.priority || left.order - right.order);
	for (const contribution of ordered) {
		const configuration = contribution.configuration;
		if (configuration.comments === null) {
			lineComment = undefined;
			blockComment = undefined;
		} else if (configuration.comments !== undefined) {
			if (configuration.comments.lineComment !== undefined) lineComment = configuration.comments.lineComment ?? undefined;
			if (configuration.comments.blockComment !== undefined) blockComment = configuration.comments.blockComment ?? undefined;
		}
		if (configuration.brackets !== undefined) brackets = configuration.brackets ?? Object.freeze([]);
		if (configuration.autoClosingPairs !== undefined) autoClosingPairs = configuration.autoClosingPairs ?? Object.freeze([]);
		if (configuration.surroundingPairs !== undefined) surroundingPairs = configuration.surroundingPairs ?? Object.freeze([]);
		if (configuration.autoCloseBefore !== undefined) autoCloseBefore = configuration.autoCloseBefore ?? "";
		if (configuration.indentationRules !== undefined) indentationRules = configuration.indentationRules ?? undefined;
		if (configuration.foldingMarkers !== undefined) foldingMarkers = configuration.foldingMarkers ?? undefined;
		if (configuration.onEnterRules !== undefined) onEnterRules = configuration.onEnterRules ?? Object.freeze([]);
		if (configuration.wordPattern !== undefined) wordPattern = configuration.wordPattern ?? undefined;
	}
	const resolvedAutoClosingPairs = autoClosingPairs ?? brackets;
	const resolvedSurroundingPairs = surroundingPairs ?? Object.freeze(resolvedAutoClosingPairs.map(pair => Object.freeze({
		open: pair.open,
		close: pair.close,
	})));
	return Object.freeze({
		languageId,
		revision,
		comments: Object.freeze({
			...(lineComment === undefined ? {} : { lineComment }),
			...(blockComment === undefined ? {} : { blockComment }),
		}),
		brackets,
		autoClosingPairs: resolvedAutoClosingPairs,
		surroundingPairs: resolvedSurroundingPairs,
		autoCloseBefore: autoCloseBefore ?? DEFAULT_LANGUAGE_AUTO_CLOSE_BEFORE,
		...(indentationRules === undefined ? {} : { indentationRules }),
		...(foldingMarkers === undefined ? {} : { foldingMarkers }),
		onEnterRules,
		...(wordPattern === undefined ? {} : { wordPattern }),
	});
}
