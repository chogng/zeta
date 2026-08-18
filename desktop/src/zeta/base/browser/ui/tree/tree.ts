export const TreeVisibility = Object.freeze({
  Hidden: "hidden",
  Visible: "visible",
  Recurse: "recurse",
} as const);

export type TreeVisibility = typeof TreeVisibility[keyof typeof TreeVisibility];
export type TreeFilterResult = boolean | TreeVisibility;

export interface TreeFilter<T> {
  filter(element: T, parentVisibility: TreeVisibility): TreeFilterResult;
}

export interface TreeSorter<T> {
  compare(left: T, right: T): number;
}

export interface TreeElement<T> {
  readonly element: T;
  readonly children?: readonly TreeElement<T>[];
  readonly collapsible?: boolean;
  readonly collapsed?: boolean;
}

export interface TreeDataSource<TInput, T> {
  readonly hasChildren?: (element: TInput | T) => boolean;
  readonly getChildren: (element: TInput | T) => Iterable<T>;
}

export interface AsyncTreeDataSource<TInput, T> {
  readonly hasChildren: (element: TInput | T) => boolean;
  readonly getChildren: (element: TInput | T) => Iterable<T> | Promise<Iterable<T>>;
  readonly getParent?: (element: T) => TInput | T;
}

export type IndexTreeLocation = readonly number[];

export type TreeIndentGuides = "none" | "onHover" | "always";

export interface TreeTwistieState {
  readonly collapsible: boolean;
  readonly expanded: boolean;
}

export type TreePointerTarget = "twistie" | "contents";

/** Structural node contract projected by `AbstractTree` into flat list rows. */
export interface AbstractTreeNode<T> {
  readonly id: string;
  readonly element: T;
  readonly parent: AbstractTreeNode<T> | undefined;
  readonly children: readonly AbstractTreeNode<T>[];
  readonly depth: number;
  readonly collapsible: boolean;
  readonly collapsed: boolean;
  readonly visible: boolean;
  readonly visibleChildIndex: number;
  readonly visibleChildrenCount: number;
}

export interface TreePointerEvent<T> {
  readonly element: T;
  readonly target: TreePointerTarget;
  readonly browserEvent: MouseEvent;
}

export interface TreeAcceptEvent<T> {
  readonly element: T;
  readonly browserEvent: KeyboardEvent;
}

export interface TreeFocusChangeEvent<T> {
  readonly element: T | undefined;
  readonly browserEvent: UIEvent | undefined;
}

export interface TreeSelectionChangeEvent<T> {
  readonly elements: readonly T[];
  readonly browserEvent: UIEvent | undefined;
}

export interface TreeCollapseRequestEvent<T> {
  readonly element: T;
  readonly expanded: boolean;
  readonly browserEvent: MouseEvent | KeyboardEvent;
}

export interface TreeActivateEvent<T> {
  readonly element: T;
  readonly browserEvent: MouseEvent | KeyboardEvent;
}
