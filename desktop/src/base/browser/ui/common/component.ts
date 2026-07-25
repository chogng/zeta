/** A small lifecycle wrapper around a browser DOM element. */
export abstract class Component<TElement extends HTMLElement = HTMLElement> {
  readonly element: TElement;
  #disposers: Array<() => void> = [];

  protected constructor(element: TElement) {
    this.element = element;
  }

  mount(container: Element): this {
    container.append(this.element);
    return this;
  }

  protected listen<T extends Event>(target: EventTarget, type: string, listener: (event: T) => void): void {
    target.addEventListener(type, listener as EventListener);
    this.#disposers.push(() => target.removeEventListener(type, listener as EventListener));
  }

  dispose(): void {
    for (const dispose of this.#disposers.splice(0)) dispose();
    this.element.remove();
  }
}
