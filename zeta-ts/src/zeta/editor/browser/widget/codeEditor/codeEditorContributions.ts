import { addDisposableListener } from '../../../../base/browser/dom.js';
import { DisposableMap, DisposableOwner, type IDisposable } from '../../../../base/common/lifecycle.js';
import { runWhenWindowIdle, scheduleAtNextAnimationFrame } from '../../../../base/browser/scheduler.js';
import { getWindow } from '../../../../base/browser/window.js';
import { type EditorSelectionController } from '../../../common/cursor/editorSelectionController.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type IInstantiationService, type SyncDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { type EditorInputController } from '../../controller/inputController.js';
import { type EditorViewport } from '../../view/editorViewport.js';

/** Controls when a widget contribution joins one CodeEditorWidget's lifetime. */
export enum CodeEditorContributionInstantiation {
	Eager = 'eager',
	AfterFirstRender = 'afterFirstRender',
	BeforeFirstInteraction = 'beforeFirstInteraction',
	Eventually = 'eventually',
	Lazy = 'lazy',
}

/** Narrow editor state exposed to a direct CodeEditorWidget contribution. */
export interface CodeEditorContributionContext {
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController;
	readonly viewport: EditorViewport;
	readonly input: EditorInputController;
	readonly placeholder: string | undefined;
}

export interface CodeEditorContribution extends IDisposable {}

export interface CodeEditorContributionDescription {
	readonly id: string;
	readonly descriptor: SyncDescriptor<CodeEditorContribution>;
	readonly instantiation: CodeEditorContributionInstantiation;
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
export class CodeEditorContributions extends DisposableOwner {
	private readonly instances = this.own(new DisposableMap<string, CodeEditorContribution>());
	private readonly pending = new Map<string, CodeEditorContributionDescription>();
	private readonly completedInstantiation = new Set<CodeEditorContributionInstantiation>();
	private context: CodeEditorContributionContext | undefined;
	private instantiationService: IInstantiationService | undefined;
	private onError: (error: unknown) => void = reportContributionError;

	constructor() {
		super();
		this.defer(() => this.pending.clear());
	}

	initialize(
		context: CodeEditorContributionContext,
		instantiationService: IInstantiationService,
		descriptions: readonly CodeEditorContributionDescription[] = getCodeEditorContributions(),
		onError?: (error: unknown) => void,
	): void {
		this.assertNotDisposed();
		if (this.context) throw new Error('Code editor contributions have already been initialized');
		if (!instantiationService || typeof instantiationService.createInstance !== 'function') throw new TypeError('Code editor contributions require an instantiation service');
		if (typeof onError === 'function') this.onError = onError;
		this.context = context;
		this.instantiationService = instantiationService;
		for (const description of descriptions) {
			if (!isValidDescription(description)) {
				throw new TypeError('Code editor contribution is invalid');
			}
			if (this.pending.has(description.id)) throw new RangeError(`Duplicate code editor contribution '${description.id}'`);
			this.pending.set(description.id, description);
		}

		this.instantiateSome(CodeEditorContributionInstantiation.Eager);
		this.own(addDisposableListener(context.viewport.element, 'pointerdown', () => this.onBeforeInteractionEvent(), true));
		this.own(addDisposableListener(context.viewport.element, 'wheel', () => this.onBeforeInteractionEvent(), true));
		this.own(addDisposableListener(context.viewport.element, 'contextmenu', () => this.onBeforeInteractionEvent(), true));
		for (const type of ['keydown', 'beforeinput', 'compositionstart', 'paste', 'cut'] as const) {
			this.own(addDisposableListener(context.input.element, type, () => this.onBeforeInteractionEvent(), true));
		}

		const targetWindow = getWindow(context.viewport.element);
		this.own(scheduleAtNextAnimationFrame(targetWindow, () => this.instantiateSome(CodeEditorContributionInstantiation.AfterFirstRender)));
		this.own(runWhenWindowIdle(targetWindow, () => this.instantiateSome(CodeEditorContributionInstantiation.Eventually), { timeoutMs: 5_000 }));
	}

	get(id: string): CodeEditorContribution | undefined {
		this.instantiateById(id);
		for (const [contributionId, instance] of this.instances) {
			if (contributionId === id) return instance;
		}
		return undefined;
	}

	onBeforeInteractionEvent(): void {
		this.instantiateSome(CodeEditorContributionInstantiation.BeforeFirstInteraction);
	}

	private instantiateSome(instantiation: CodeEditorContributionInstantiation): void {
		if (this.isDisposed || this.completedInstantiation.has(instantiation)) return;
		this.completedInstantiation.add(instantiation);
		const pending = [...this.pending.values()].filter(description => description.instantiation === instantiation);
		for (const description of pending) this.instantiateById(description.id);
	}

	private instantiateById(id: string): void {
		if (this.isDisposed) return;
		const description = this.pending.get(id);
		if (!description) return;
		this.pending.delete(id);
		const context = this.context;
		const instantiationService = this.instantiationService;
		if (!context || !instantiationService) throw new Error('Code editor contributions have not been initialized');
		try {
			const instance = instantiationService.createInstance(description.descriptor, context);
			if (!instance || typeof instance.dispose !== 'function') throw new TypeError(`Code editor contribution '${id}' did not return a disposable`);
			this.instances.set(id, instance);
		} catch (error) {
			if (description.instantiation === CodeEditorContributionInstantiation.Eager) throw error;
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

function isInstantiation(value: CodeEditorContributionInstantiation): boolean {
	return value === CodeEditorContributionInstantiation.Eager
		|| value === CodeEditorContributionInstantiation.AfterFirstRender
		|| value === CodeEditorContributionInstantiation.BeforeFirstInteraction
		|| value === CodeEditorContributionInstantiation.Eventually
		|| value === CodeEditorContributionInstantiation.Lazy;
}

function reportContributionError(error: unknown): void {
	console.error('Code editor contribution failed', error);
}
