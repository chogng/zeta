export interface IDimension {
  readonly width: number;
  readonly height: number;
}

export class Dimension implements IDimension {
  static readonly Zero = new Dimension(0, 0);

  constructor(
    readonly width: number,
    readonly height: number,
  ) {}

  with(
    width = this.width,
    height = this.height,
  ): Dimension {
    return width === this.width && height === this.height
      ? this
      : new Dimension(width, height);
  }

  static equals(
    left: IDimension | undefined,
    right: IDimension | undefined,
  ): boolean {
    return left === right ||
      Boolean(left && right &&
        left.width === right.width &&
        left.height === right.height);
  }
}

export interface IPosition {
  readonly left: number;
  readonly top: number;
}

export interface IRectangle extends IPosition, IDimension {}

export function getClientArea(element: HTMLElement): Dimension {
  const targetWindow = getOwnerWindow(element);
  if (element === targetWindow.document.body) {
    const viewport = targetWindow.visualViewport;
    return new Dimension(
      viewport?.width ?? targetWindow.innerWidth,
      viewport?.height ?? targetWindow.innerHeight,
    );
  }
  return new Dimension(element.clientWidth, element.clientHeight);
}

export function getViewport(targetWindow: Window): IRectangle {
  const viewport = targetWindow.visualViewport;
  return {
    left: viewport?.offsetLeft ?? 0,
    top: viewport?.offsetTop ?? 0,
    width: viewport?.width ?? targetWindow.innerWidth,
    height: viewport?.height ?? targetWindow.innerHeight,
  };
}

export function getDomNodePagePosition(
  element: HTMLElement,
): IRectangle {
  const bounds = element.getBoundingClientRect();
  const targetWindow = getOwnerWindow(element);
  return {
    left: bounds.left + targetWindow.scrollX,
    top: bounds.top + targetWindow.scrollY,
    width: bounds.width,
    height: bounds.height,
  };
}

export function getContentSize(element: HTMLElement): Dimension {
  const style = getOwnerWindow(element).getComputedStyle(element);
  return new Dimension(
    element.offsetWidth -
      pixels(style.borderLeftWidth) -
      pixels(style.borderRightWidth) -
      pixels(style.paddingLeft) -
      pixels(style.paddingRight),
    element.offsetHeight -
      pixels(style.borderTopWidth) -
      pixels(style.borderBottomWidth) -
      pixels(style.paddingTop) -
      pixels(style.paddingBottom),
  );
}

export function getTotalSize(element: HTMLElement): Dimension {
  const style = getOwnerWindow(element).getComputedStyle(element);
  return new Dimension(
    element.offsetWidth +
      pixels(style.marginLeft) +
      pixels(style.marginRight),
    element.offsetHeight +
      pixels(style.marginTop) +
      pixels(style.marginBottom),
  );
}

function pixels(value: string): number {
  return Number.parseFloat(value) || 0;
}

function getOwnerWindow(element: HTMLElement): Window {
  const targetWindow = element.ownerDocument.defaultView;
  if (!targetWindow) throw new Error("DOM geometry requires an element with an owning window");
  return targetWindow;
}
