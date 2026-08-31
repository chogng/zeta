import { addDisposableListener, getWindow } from '../../../../base/browser/dom.js';
import { DisposableMap, Disposable, type IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { runWhenWindowIdle } from '../../../../base/browser/dom.js';
import { scheduleAtNextAnimationFrame } from '../../../../base/browser/scheduler.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type IInstantiationService, type ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { type View } from '../../view.js';
import { type ViewController } from '../../view/viewController.js';
import { type CodeEditorWidget } from './codeEditorWidget.js';
import { EditorContributionInstantiation } from '../../editorExtensions.js';

/** Narrow editor state exposed to a direct CodeEditorWidget contribution. */
export interface CodeEditorContributionContext {
	readonly editor: CodeEditorWidget;
	readonly model: TextModel;
	readonly selectionController: CursorsController;
	readonly viewport: View;
	readonly view: ViewController;
	readonly placeholder: string | undefined;
}

export interface CodeEditorContribution extends IDisposable {}

export interface CodeEditorContributionDescription {
	readonly id: string;
	readonly descriptor: ServiceConstructionDescriptor<CodeEditorContribution>;
	readonly instantiation: EditorContributionInstantiation;
}

interface PendingCodeEditorContribution {
	readonly context: unknown;
	readonly description: CodeEditorContributionDescription;
}

const contributions: CodeEditorContributionDescription[] = [];
const contributionIds = new Set<string>();

/** Registers a widget contribution for future CodeEditorWidget instances. */
export function registerCodeEditorContribution(contribution: CodeEditorContributionDescription): void {
	if (!isValidDescription(contribution)) {
		throw new TypeError('Code editor contribution is invalid');
	}
	if (contributionIds.has(contribution.id)) throw new RangeError(`Duplicate code editor contribution '${contribution.id}'`);
	contributionIds.add(contribution.id);
	contributions.push(contribution);
}

export function getCodeEditorContributions(): readonly CodeEditorContributionDescription[] {
	return contributions.slice();
}

/** Owns one CodeEditorWidget's contribution instances and their staged creation. */
export class CodeEditorContributions extends Disposable {
	private readonly instances = this._register(new DisposableMap<string, CodeEditorContribution>());
	private readonly pending = new Map<string, PendingCodeEditorContribution>();
	private readonly completedInstantiation = new Set<EditorContributionInstantiation>();
	private instantiationService: IInstantiationService | undefined;
	private onError: (error: unknown) => void = reportContributionError;

	constructor() {
		super();
		this._register(toDisposable(() => this.pending.clear()));
	}

	initialize(
		context: CodeEditorContributionContext,
		instantiationService: IInstantiationService,
		descriptions: readonly CodeEditorContributionDescription[] = getCodeEditorContributions(),
		onError?: (error: unknown) => void,
	): void {
		this.assertNotDisposed();
		if (this.instantiationService) throw new Error('Code editor contributions have already been initialized');
		if (!instantiationService || typeof instantiationService.createInstance !== 'function') throw new TypeError('Code editor contributions require an instantiation service');
		if (typeof onError === 'function') this.onError = onError;
		this.instantiationService = instantiationService;
		this.add(context, descriptions);
		this._register(addDisposableListener(context.viewport.element, 'pointerdown', () => this.onBeforeInteractionEvent(), true));
		this._register(addDisposableListener(context.viewport.element, 'wheel', () => this.onBeforeInteractionEvent(), true));
		this._register(addDisposableListener(context.viewport.element, 'contextmenu', () => this.onBeforeInteractionEvent(), true));
		for (const type of ['keydown', 'beforeinput', 'compositionstart', 'paste', 'cut'] as const) {
			this._register(addDisposableListener(context.view.element, type, () => this.onBeforeInteractionEvent(), true));
		}

		const targetWindow = getWindow(context.viewport.element);
		this._register(scheduleAtNextAnimationFrame(targetWindow, () => this.instantiateSome(EditorContributionInstantiation.AfterFirstRender)));
		this._register(runWhenWindowIdle(targetWindow, () => this.instantiateSome(EditorContributionInstantiation.Eventually), 5_000));
	}

	/** Adds another contribution group that shares this widget's instantiation phases and lifetime. */
	add<TContext>(context: TContext, descriptions: readonly CodeEditorContributionDescription[]): void {
		this.assertNotDisposed();
		if (!this.instantiationService) throw new Error('Code editor contributions have not been initialized');
		const incomingIds = new Set<string>();
		for (const description of descriptions) {
			if (!isValidDescription(description)) throw new TypeError('Code editor contribution is invalid');
			if (incomingIds.has(description.id) || this.pending.has(description.id) || this.instances.has(description.id)) {
				throw new RangeError(`Duplicate code editor contribution '${description.id}'`);
			}
			incomingIds.add(description.id);
		}
		for (const description of descriptions) {
			this.pending.set(description.id, { context, description });
		}
		for (const description of descriptions) {
			if (this.completedInstantiation.has(description.instantiation)) this.instantiateById(description.id);
		}
		this.instantiateSome(EditorContributionInstantiation.Eager);
	}

	get(id: string): CodeEditorContribution | undefined {
		this.instantiateById(id);
		for (const [contributionId, instance] of this.instances) {
			if (contributionId === id) return instance;
		}
		return undefined;
	}

	onBeforeInteractionEvent(): void {
		this.instantiateSome(EditorContributionInstantiation.BeforeFirstInteraction);
	}

	private instantiateSome(instantiation: EditorContributionInstantiation): void {
		if (this.isDisposed || this.completedInstantiation.has(instantiation)) return;
		this.completedInstantiation.add(instantiation);
		const pending = [...this.pending.values()].filter(value => value.description.instantiation === instantiation);
		for (const value of pending) this.instantiateById(value.description.id);
	}

	private instantiateById(id: string): void {
		if (this.isDisposed) return;
		const pending = this.pending.get(id);
		if (!pending) return;
		this.pending.delete(id);
		const instantiationService = this.instantiationService;
		if (!instantiationService) throw new Error('Code editor contributions have not been initialized');
		try {
			const instance = instantiationService.createInstance(pending.description.descriptor, pending.context);
			if (!instance || typeof instance.dispose !== 'function') throw new TypeError(`Code editor contribution '${id}' did not return a disposable`);
			this.instances.set(id, instance);
		} catch (error) {
			if (pending.description.instantiation === EditorContributionInstantiation.Eager) throw error;
			this.onError(error);
		}
	}
}

function isValidDescription(description: CodeEditorContributionDescription): boolean {
	return Boolean(
		description
			&& typeof description.id === 'string'
			&& description.id.trim().length > 0
			&& description.descriptor?.ctor
			&& isInstantiation(description.instantiation),
	);
}

function isInstantiation(value: EditorContributionInstantiation): boolean {
	return value === EditorContributionInstantiation.Eager
		|| value === EditorContributionInstantiation.AfterFirstRender
		|| value === EditorContributionInstantiation.BeforeFirstInteraction
		|| value === EditorContributionInstantiation.Eventually
		|| value === EditorContributionInstantiation.Lazy;
}

function reportContributionError(error: unknown): void {
	console.error('Code editor contribution failed', error);
}
