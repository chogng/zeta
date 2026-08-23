import { Hover, type HoverContent } from "../../../base/browser/ui/hover/hover.js";
import { addDisposableListener, isHTMLElement } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../configuration/common/configurationService.js";
import type { IContextMenuService } from "../../contextview/browser/contextMenu.js";
import type { IContextViewService } from "../../contextview/browser/contextView.js";
import { HoverConfiguration, type HoverDelayMode, type HoverSetupOptions, type IHoverService, type IManagedHover } from "../common/hoverService.js";

const InstantHoverWindowMs = 200;
const PointerHoverResumeDistance = 2;

/** Browser implementation of the window-scoped Workbench Hover policy. */
export class HoverService extends DisposableOwner implements IHoverService {
	private readonly managedHovers = new Set<ManagedHover>();
	private readonly configurationService: IConfigurationService;
	private readonly contextViewService: IContextViewService;
	private activeHover: ManagedHover | undefined;
	private contextMenuVisible = false;
	private pointerHoverSuppressed = false;
	private pointerActivationPosition: { readonly x: number; readonly y: number } | undefined;
	private lastGroupId: string | undefined;
	private lastHideTime = 0;

	constructor(configurationService: IConfigurationService, contextViewService: IContextViewService, contextMenuService: IContextMenuService) {
		super();
		this.configurationService = configurationService;
		this.contextViewService = contextViewService;
		const ownerDocument = contextViewService.container.ownerDocument;
		this.own(addDisposableListener(ownerDocument, "pointerdown", (event) => {
			if (isHTMLElement(event.target) && event.target.closest(".zeta-hover")) return;
			this.pointerHoverSuppressed = true;
			this.pointerActivationPosition = { x: event.clientX, y: event.clientY };
			this.hideHover();
			this.lastGroupId = undefined;
			this.lastHideTime = 0;
		}, true));
		this.own(addDisposableListener(ownerDocument, "pointermove", (event) => {
			const activationPosition = this.pointerActivationPosition;
			if (event.buttons !== 0 || !activationPosition) return;
			const deltaX = event.clientX - activationPosition.x;
			const deltaY = event.clientY - activationPosition.y;
			if (deltaX * deltaX + deltaY * deltaY <= PointerHoverResumeDistance * PointerHoverResumeDistance) return;
			this.pointerHoverSuppressed = false;
			this.pointerActivationPosition = undefined;
		}, true));
		this.own(contextMenuService.onDidShowContextMenu(() => {
			this.contextMenuVisible = true;
			this.hideHover();
		}));
		this.own(contextMenuService.onDidHideContextMenu(() => {
			this.contextMenuVisible = false;
		}));
		this.defer(() => {
			for (const hover of [...this.managedHovers]) hover.dispose();
		});
	}

	setupHover(options: HoverSetupOptions): IManagedHover {
		let managed!: ManagedHover;
		managed = new ManagedHover({
			hover: new Hover({
				target: options.target,
				content: options.content,
				delayMs: () => this.resolveDelay(options.delay, options.groupId),
				persistence: options.persistence,
				enabled: () => !this.contextMenuVisible,
				pointerHoverEnabled: () => !this.pointerHoverSuppressed,
				anchorAlignment: options.anchorAlignment,
				anchorAxisAlignment: options.anchorAxisAlignment,
				anchorPosition: options.anchorPosition,
				gap: options.gap,
				contextViewProvider: this.contextViewService,
			}),
			groupId: options.groupId,
			onDidShow: () => this.didShow(managed),
			onDidHide: () => this.didHide(managed),
			onDispose: () => this.release(managed),
		});
		this.managedHovers.add(managed);
		return managed;
	}

	showHover(options: HoverSetupOptions): IManagedHover {
		const hover = this.setupHover({ ...options, delay: "instant" });
		hover.show();
		return hover;
	}

	hideHover(): void {
		this.activeHover?.hide();
	}

	private resolveDelay(mode: HoverDelayMode | undefined, groupId: string | undefined): number {
		if (mode === "instant" || this.shouldSkipDelay(groupId)) return 0;
		return this.configurationService.getValue(
			mode === "reduced"
				? HoverConfiguration.reducedDelay
				: HoverConfiguration.delay,
		);
	}

	private shouldSkipDelay(groupId: string | undefined): boolean {
		if (groupId === undefined) return false;
		if (this.activeHover?.groupId === groupId) return true;
		return this.lastGroupId === groupId &&
			Date.now() - this.lastHideTime <= InstantHoverWindowMs;
	}

	private didShow(hover: ManagedHover): void {
		this.activeHover = hover;
	}

	private didHide(hover: ManagedHover): void {
		if (this.activeHover !== hover) return;
		this.activeHover = undefined;
		this.lastGroupId = hover.groupId;
		this.lastHideTime = Date.now();
	}

	private release(hover: ManagedHover): void {
		this.managedHovers.delete(hover);
		this.didHide(hover);
	}
}

interface ManagedHoverOptions {
	readonly hover: Hover;
	readonly groupId?: string;
	readonly onDidShow: () => void;
	readonly onDidHide: () => void;
	readonly onDispose: () => void;
}

class ManagedHover extends DisposableOwner implements IManagedHover {
	readonly groupId: string | undefined;
	private readonly hover: Hover;

	constructor(options: ManagedHoverOptions) {
		super();
		this.groupId = options.groupId;
		this.hover = this.own(options.hover);
		this.own(this.hover.onDidShow(options.onDidShow));
		this.own(this.hover.onDidHide(options.onDidHide));
		this.defer(options.onDispose);
	}

	get visible(): boolean {
		return this.hover.visible;
	}

	show(): void {
		this.hover.show();
	}

	hide(): void {
		this.hover.hide();
	}

	update(content: HoverContent): void {
		this.hover.update(content);
	}
}
