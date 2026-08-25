import { Emitter, type Event } from '../../../base/common/event.js';
import { WorkbenchPart } from "../part.js";
import { PaneComposite } from "./views/paneComposite.js";

/**
 * Workbench Part that retains and activates one PaneComposite at a time.
 *
 * The shared content area hosts the active Composite. Pane-like subclasses
 * add their standard title and CompositeBar through PaneCompositePart.
 */
export abstract class CompositePart extends WorkbenchPart {
	private readonly activeCompositeChangeEmitter = this.own(new Emitter<string>());
	private readonly composites = new Map<string, PaneComposite>();
	private activeComposite: PaneComposite | undefined;
	readonly onDidChangeActiveComposite: Event<string> = this.activeCompositeChangeEmitter.event;

	protected constructor(container: HTMLElement, id: string) {
		super(container, id);
		this.contentDomNode.classList.add("zeta-composite-content");
		this.defer(() => this.composites.clear());
	}

	addComposite(composite: PaneComposite): void {
		if (this.composites.has(composite.id)) {
			throw new Error(`Composite already exists in Part: ${composite.id}`);
		}
		this.composites.set(composite.id, this.own(composite));
		composite.setVisible(false);
	}

	getComposite(compositeId: string): PaneComposite | undefined {
		return this.composites.get(compositeId);
	}

	showComposite(compositeId: string): void {
		const composite = this.composites.get(compositeId);
		if (!composite) {
			throw new Error(`Composite is not available in Part: ${compositeId}`);
		}
		if (this.activeComposite === composite) return;
		if (this.activeComposite) {
			this.activeComposite.setVisible(false);
			this.activeComposite.element.remove();
		}
		this.activeComposite = composite;
		this.contentDomNode.append(composite.element);
		composite.setVisible(true);
		this.activeCompositeChangeEmitter.fire(composite.id);
	}

	get activeCompositeId(): string | undefined {
		return this.activeComposite?.id;
	}
}
