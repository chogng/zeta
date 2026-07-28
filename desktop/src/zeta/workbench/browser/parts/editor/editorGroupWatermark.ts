import {
  KeybindingLabel,
} from "../../../../base/browser/ui/keybindinglabel/keybindinglabel.js";
import {
  Emitter,
  type Event,
} from "../../../../base/common/event.js";
import {
  DisposableOwner,
  type IDisposable,
  ResettableDisposableGroup,
  toDisposable,
} from "../../../../base/common/lifecycle.js";
import type {
  CommandId,
} from "../../../../platform/commands/common/commands.js";
import type {
  IKeybindingService,
} from "../../../../platform/keybinding/common/keybinding.js";

/** One command presented while an editor group has no active editor. */
export interface IEditorGroupWatermarkEntry {
  readonly id: string;
  readonly label: string;
  readonly command: CommandId;
}

class EditorGroupWatermarkRegistry {
  readonly #entries = new Map<string, IEditorGroupWatermarkEntry>();
  readonly #onDidChange = new Emitter<void>();

  readonly onDidChange: Event<void> = this.#onDidChange.event;

  register(entry: IEditorGroupWatermarkEntry): IDisposable {
    if (this.#entries.has(entry.id)) {
      throw new TypeError(
        `Editor group watermark entry '${entry.id}' is already registered`,
      );
    }
    this.#entries.set(entry.id, entry);
    this.#onDidChange.fire();
    return toDisposable(() => {
      if (this.#entries.delete(entry.id)) {
        this.#onDidChange.fire();
      }
    });
  }

  getEntries(): readonly IEditorGroupWatermarkEntry[] {
    return [...this.#entries.values()];
  }
}

/** Registry populated by command contributions shown in the empty editor. */
export const EditorGroupWatermarkEntries =
  new EditorGroupWatermarkRegistry();

/** Renders command shortcuts when an editor group has no active editor. */
export class EditorGroupWatermark extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #rendered = this.own(new ResettableDisposableGroup());
  readonly #keybindingService: IKeybindingService;

  constructor(
    ownerDocument: Document,
    keybindingService: IKeybindingService,
  ) {
    super();
    this.#keybindingService = keybindingService;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-editor-group-watermark";
    this.element.setAttribute("aria-label", "Editor shortcuts");
    this.defer(() => this.element.remove());
    this.own(EditorGroupWatermarkEntries.onDidChange(() => this.#render()));
    this.own(
      this.#keybindingService.onDidUpdateKeybindings(() => this.#render()),
    );
    this.#render();
  }

  #render(): void {
    this.#rendered.clear();
    const ownerDocument = this.element.ownerDocument;
    const rows = EditorGroupWatermarkEntries.getEntries()
      .flatMap((entry) => {
        const keybinding =
          this.#keybindingService.lookupKeybinding(entry.command);
        if (!keybinding) return [];

        const row = ownerDocument.createElement("div");
        row.className = "zeta-editor-group-watermark-entry";
        const label = ownerDocument.createElement("span");
        label.className = "zeta-editor-group-watermark-label";
        label.textContent = entry.label;
        const shortcut = this.#rendered.add(new KeybindingLabel({
          keybinding,
          ownerDocument,
        }));
        row.append(label, shortcut.element);
        return [row];
      });
    this.element.replaceChildren(...rows);
  }
}
