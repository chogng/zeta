import "./rulers.css";
import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type TextMeasurer } from "../../measurement/fontMetrics.js";
import { type EditorViewPart } from "../viewPart.js";

/** One 1-based editor column at which a vertical guide is rendered. */
export interface EditorRuler {
  readonly column: number;
  readonly color?: string;
}

export interface RulersPartOptions {
  readonly container: HTMLElement;
  readonly textMeasurer: TextMeasurer;
  readonly readTextLeft: () => number;
  readonly rulers?: readonly EditorRuler[];
}

/** Projects configured column guides into the scrollable editor content. */
export class RulersPart extends DisposableOwner implements EditorViewPart {
  readonly element: HTMLDivElement;
  private readonly textMeasurer: TextMeasurer;
  private readonly readTextLeft: () => number;
  private readonly rulers: readonly EditorRuler[];
  private readonly renderedRulers: HTMLDivElement[] = [];

  constructor(options: RulersPartOptions) {
    super();
    this.textMeasurer = options.textMeasurer;
    this.readTextLeft = options.readTextLeft;
    this.rulers = Object.freeze([...(options.rulers ?? [])].map(validateRuler));
    this.element = h(options.container.ownerDocument, "div");
    this.element.className = "aster-editor-rulers";
    this.element.setAttribute("role", "presentation");
    this.element.setAttribute("aria-hidden", "true");
    options.container.append(this.element);
    this.defer(() => this.element.remove());
  }

  render(layout: EditorViewportLayout): void {
    this.element.style.width = `${layout.contentSize.width}px`;
    this.element.style.height = `${Math.min(layout.contentSize.height, 1_000_000)}px`;
    if (this.renderedRulers.length !== this.rulers.length) {
      const fragment = createFragment(this.element.ownerDocument);
      this.renderedRulers.length = 0;
      for (const ruler of this.rulers) {
        const element = h(this.element.ownerDocument, "div");
        element.className = "aster-editor-ruler";
        fragment.append(element);
        this.renderedRulers.push(element);
      }
      reset(this.element, fragment);
    }
    for (let index = 0; index < this.rulers.length; index += 1) {
      const ruler = this.rulers[index]!;
      const element = this.renderedRulers[index]!;
      element.style.left = `${this.readTextLeft() + this.textMeasurer.measureLineWidth("0".repeat(ruler.column))}px`;
      element.style.height = `${Math.min(layout.contentSize.height, 1_000_000)}px`;
      element.style.boxShadow = ruler.color
        ? `1px 0 0 0 ${ruler.color} inset`
        : "";
    }
  }
}

function validateRuler(ruler: EditorRuler): EditorRuler {
  if (!ruler || !Number.isSafeInteger(ruler.column) || ruler.column < 1) {
    throw new RangeError("Aster ruler columns must be positive safe integers");
  }
  if (ruler.color !== undefined && (typeof ruler.color !== "string" || ruler.color.trim().length === 0)) {
    throw new TypeError("Aster ruler colors must be non-empty strings");
  }
  return Object.freeze({
    column: ruler.column,
    ...(ruler.color === undefined ? {} : { color: ruler.color }),
  });
}
