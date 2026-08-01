import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Tree } from "../../browser/ui/tree/tree.js";

interface TestNode {
  readonly id: string;
  readonly label: string;
  expanded: boolean;
  readonly children?: readonly TestNode[];
}

test("Tree projects hierarchy, expansion, and indentation guides", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const child: TestNode = { id: "child", label: "Child", expanded: false };
  const parent: TestNode = { id: "parent", label: "Parent", expanded: false, children: [child] };
  const sibling: TestNode = { id: "sibling", label: "Sibling", expanded: false };
  const tree = createTree(dom, [parent, sibling]);
  const activated: string[] = [];
  tree.onDidActivate(({ element }) => activated.push(element.id));

  assert.equal(tree.element.getAttribute("role"), "tree");
  assert.equal(tree.element.getAttribute("aria-label"), "Test tree");
  assert.ok(tree.element.classList.contains("zeta-tree-indent-guides-always"));
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 2);
  const parentRow = tree.element.querySelector<HTMLButtonElement>("[data-tree-id='parent']");
  assert.ok(parentRow);
  assert.equal(parentRow.getAttribute("aria-expanded"), "false");

  parentRow.click();
  assert.deepEqual(activated, ["parent"]);
  parent.expanded = true;
  tree.items = [parent, sibling];
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 3);
  assert.equal(tree.element.querySelector(".zeta-tree-group")?.getAttribute("role"), "group");
  assert.equal(tree.element.querySelector("[data-tree-id='child']")?.getAttribute("aria-level"), "2");
  assert.equal(tree.element.querySelector("[data-tree-id='child']")?.querySelectorAll(".zeta-tree-indent-guide").length, 2);
  assert.equal(tree.element.querySelector("[data-tree-id='parent']")?.getAttribute("aria-expanded"), "true");

  tree.dispose();
  dom.window.close();
});

test("Tree owns roving focus and arrow navigation", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const first: TestNode = { id: "first", label: "First", expanded: false };
  const second: TestNode = { id: "second", label: "Second", expanded: false };
  const tree = createTree(dom, [first, second]);
  dom.window.document.body.append(tree.element);
  const firstRow = tree.element.querySelector<HTMLButtonElement>("[data-tree-id='first']");
  const secondRow = tree.element.querySelector<HTMLButtonElement>("[data-tree-id='second']");
  assert.ok(firstRow);
  assert.ok(secondRow);
  assert.equal(firstRow.tabIndex, 0);
  assert.equal(secondRow.tabIndex, -1);

  firstRow.focus();
  firstRow.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowDown" }));
  assert.equal(dom.window.document.activeElement, secondRow);
  assert.equal(firstRow.tabIndex, -1);
  assert.equal(secondRow.tabIndex, 0);

  secondRow.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "Home" }));
  assert.equal(dom.window.document.activeElement, firstRow);

  tree.dispose();
  dom.window.close();
});

function createTree(dom: JSDOM, items: readonly TestNode[]): Tree<TestNode> {
  const tree = new Tree<TestNode>({
    ownerDocument: dom.window.document,
    ariaLabel: "Test tree",
    indentGuides: "always",
    getId: (node) => node.id,
    getChildren: (node) => node.children,
    isCollapsible: (node) => node.children !== undefined,
    isExpanded: (node) => node.expanded,
    renderElement: (node) => {
      const label = dom.window.document.createElement("span");
      label.textContent = node.label;
      return label;
    },
  });
  tree.items = items;
  return tree;
}
