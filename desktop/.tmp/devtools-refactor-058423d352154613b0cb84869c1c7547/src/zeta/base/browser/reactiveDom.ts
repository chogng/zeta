import type { Event } from "../common/event.js";
import {
  DisposableSlot,
  DisposableStore,
  type IDisposable,
} from "../common/lifecycle.js";

/** Minimal readable state contract consumed by reactive DOM bindings. */
export interface ReadableValue<T> {
  get(): T;
  readonly onDidChange: Event<T>;
}

export interface RenderedDom {
  readonly children: readonly (Node | string)[];
  readonly registration?: IDisposable;
}

export function bindText(
  node: Node,
  source: ReadableValue<string>,
): IDisposable {
  const update = (value = source.get()): void => {
    node.textContent = value;
  };
  update();
  return source.onDidChange(update);
}

export function bindAttribute(
  element: Element,
  name: string,
  source: ReadableValue<string | undefined>,
): IDisposable {
  const update = (value = source.get()): void => {
    if (value === undefined) element.removeAttribute(name);
    else element.setAttribute(name, value);
  };
  update();
  return source.onDidChange(update);
}

export function bindClass(
  element: Element,
  className: string,
  source: ReadableValue<boolean>,
): IDisposable {
  const update = (value = source.get()): void => {
    element.classList.toggle(className, value);
  };
  update();
  return source.onDidChange(update);
}

/**
 * Re-renders a container from readable state and disposes resources created by
 * the previous render before replacing its nodes.
 */
export function bindChildren<T>(
  container: Element,
  source: ReadableValue<T>,
  render: (value: T) => RenderedDom,
): IDisposable {
  const store = new DisposableStore();
  const renderedRegistration = store.add(
    new DisposableSlot<IDisposable>(),
  );
  const update = (value = source.get()): void => {
    const rendered = render(value);
    renderedRegistration.replace(rendered.registration);
    container.replaceChildren(...rendered.children);
  };
  update();
  store.add(source.onDidChange(update));
  return store;
}
