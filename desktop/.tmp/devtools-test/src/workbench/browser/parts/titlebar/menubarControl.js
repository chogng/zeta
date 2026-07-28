import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { SubmenuAction } from "../../../../base/common/actions.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
/** Compact application-menu trigger used by web, Windows, and Linux. */
export class BrowserMenubarControl extends DisposableOwner {
    #menu;
    #contextMenuService;
    #button;
    #active = false;
    element;
    constructor(menuService, contextMenuService, ownerDocument) {
        super();
        this.#contextMenuService = contextMenuService;
        this.element = ownerDocument.createElement("nav");
        this.element.className = "zeta-menubar";
        this.element.setAttribute("aria-label", "Application menu");
        this.defer(() => this.element.remove());
        this.#menu = this.own(menuService.createMenu(MenuId.MenubarMainMenu));
        this.#button = this.own(new Button({
            label: "Application menu",
            icon: LxIcon.menu,
            ownerDocument,
            onClick: () => this.#toggleMenu(),
        }));
        this.#button.element.classList.add("zeta-menubar-item");
        this.#button.element.setAttribute("aria-haspopup", "menu");
        this.#button.element.setAttribute("aria-expanded", "false");
        this.element.append(this.#button.element);
        this.own(this.#menu.onDidChange(() => {
            if (this.#active)
                this.#contextMenuService.hideContextMenu();
        }));
        this.own(addDisposableListener(this.#button.element, "keydown", (event) => {
            if (event.isComposing ||
                event.altKey ||
                event.ctrlKey ||
                event.metaKey) {
                return;
            }
            if (event.key === "ArrowDown" || event.key === "Enter") {
                if (!this.#active)
                    this.#showMenu();
            }
            else if (event.key === "Escape" && this.#active) {
                this.#contextMenuService.hideContextMenu();
            }
            else {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
        }));
        this.defer(() => {
            if (this.#active)
                this.#contextMenuService.hideContextMenu();
        });
    }
    #toggleMenu() {
        if (this.#active) {
            this.#contextMenuService.hideContextMenu();
            return;
        }
        this.#showMenu();
    }
    #showMenu() {
        const actions = this.#menu.getActions({
            preserveEmptySubmenus: true,
        })
            .flatMap(([, groupActions]) => groupActions)
            .filter((action) => action instanceof SubmenuAction);
        if (actions.length === 0)
            return;
        this.#active = true;
        this.#button.element.classList.add("active");
        this.#button.element.setAttribute("aria-expanded", "true");
        this.#contextMenuService.showContextMenu({
            anchor: this.#button.element,
            actions,
            onHide: () => {
                this.#active = false;
                this.#button.element.classList.remove("active");
                this.#button.element.setAttribute("aria-expanded", "false");
            },
        });
    }
}
