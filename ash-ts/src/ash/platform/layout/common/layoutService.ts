import type { IDimension } from "../../../base/browser/dom.js";
import type { Event } from "../../../base/common/event.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** The offsets required when positioning overlays inside a Workbench container. */
export interface ILayoutOffsetInfo {
	readonly top: number;
	readonly quickInputTop: number;
}

/** A generic browser-container layout event. */
export interface ILayoutContainerEvent {
	readonly container: HTMLElement;
	readonly dimension: IDimension;
}

/**
 * Owns the geometry and focus boundary of one or more application containers.
 *
 * Implementations observe container size changes and publish layout events after
 * the container's dimensions are known. Product-specific topology and
 * persistence remain above this contract.
 */
export interface ILayoutService {
	readonly onDidLayoutMainContainer: Event<IDimension>;
	readonly onDidLayoutContainer: Event<ILayoutContainerEvent>;
	readonly onDidLayoutActiveContainer: Event<IDimension>;
	readonly onDidChangeActiveContainer: Event<void>;

	readonly mainContainerDimension: IDimension;
	readonly activeContainerDimension: IDimension;
	readonly mainContainer: HTMLElement;
	readonly activeContainer: HTMLElement;
	readonly containers: Iterable<HTMLElement>;

	getContainer(targetWindow: Window): HTMLElement;
	whenContainerStylesLoaded(targetWindow: Window): Promise<void> | undefined;

	readonly mainContainerOffset: ILayoutOffsetInfo;
	readonly activeContainerOffset: ILayoutOffsetInfo;

	/** Focuses the primary component hosted by the active container. */
	focus(): void;
}

export const ILayoutService = createServiceIdentifier<ILayoutService>(
	"layoutService",
);
