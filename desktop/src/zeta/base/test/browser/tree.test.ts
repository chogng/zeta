import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { ObjectTree } from "../../browser/ui/tree/objectTree.js";
import { ObjectTreeModel } from "../../browser/ui/tree/objectTreeModel.js";
import { Tree, TreeVisibility } from "../../browser/ui/tree/tree.js";

interface TestNode {
  readonly id: string;
  readonly label: string;
  expanded: boolean;
  readonly children?: readonly TestNode[];
  readonly collapsible?: boolean;
  readonly collapsed?: boolean;
}

test("ObjectTreeModel owns hierarchy, local replacement, and collapse state", () => {
  const child: TestNode = { id: "child", label: "Child", expanded: false };
  const parent: TestNode = { id: "parent", label: "Parent", expanded: false, children: [child] };
  const model = new ObjectTreeModel<TestNode>({ identityProvider: { getId: (node) => node.id } });
  const changes: string[] = [];
  model.onDidChange((event) => changes.push(event.kind));

  model.setChildren([{ element: parent, children: [{ element: child }] }]);
  assert.equal(model.getNode("child")?.element, child);
  assert.equal(model.getParent("child")?.element, parent);
  assert.equal(model.getNode("child")?.depth, 2);
  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["parent", "child"]);
  assert.equal(model.collapse("parent"), true);
  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["parent"]);
  model.setChildren([{ element: { id: "parent", label: "Updated parent", expanded: false }, children: [{ element: child }] }]);
  assert.equal(model.getNode("parent")?.collapsed, true);
  model.setNodeChildren("parent", [{ element: { id: "replacement", label: "Replacement", expanded: false } }]);
  assert.equal(model.getNode("child"), undefined);
  assert.equal(model.getParent("replacement")?.id, "parent");
  assert.throws(() => model.setChildren([
    { element: { id: "duplicate", label: "First", expanded: false } },
    { element: { id: "duplicate", label: "Second", expanded: false } },
  ]), /Duplicate tree node ID/);
  assert.equal(model.getNode("replacement")?.element.label, "Replacement");
  assert.deepEqual(changes, ["structure", "collapse", "structure", "structure"]);
  model.dispose();
});

test("ObjectTreeModel keeps local updates atomic and expands ancestors", () => {
  const model = new ObjectTreeModel<TestNode>({
    defaultCollapseState: "collapsed",
    identityProvider: { getId: (node) => node.id },
  });
  model.setChildren([{
    element: { id: "root", label: "Root", expanded: false },
    children: [{
      element: { id: "parent", label: "Parent", expanded: false },
      children: [{ element: { id: "leaf", label: "Leaf", expanded: false } }],
    }],
  }]);

  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["root"]);
  assert.equal(model.expandTo("leaf"), true);
  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["root", "parent", "leaf"]);
  assert.equal(model.collapseRecursive("root"), true);
  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["root"]);
  assert.throws(() => model.setNodeChildren("parent", [
    { element: { id: "duplicate", label: "First", expanded: false } },
    { element: { id: "duplicate", label: "Second", expanded: false } },
  ]), /Duplicate tree node ID/);
  assert.equal(model.getNode("leaf")?.element.label, "Leaf");
  model.dispose();
});

test("ObjectTreeModel filters recursively and sorts every level", () => {
  const model = new ObjectTreeModel<TestNode>({
    identityProvider: { getId: (node) => node.id },
    sorter: { compare: (left, right) => left.label.localeCompare(right.label) },
    filter: {
      filter: (node) => node.id.endsWith("-group") ? TreeVisibility.Recurse : node.label.includes("Match"),
    },
  });
  model.setChildren([
    { element: { id: "z-group", label: "Z group", expanded: false }, children: [{ element: { id: "match-b", label: "Match B", expanded: false } }] },
    { element: { id: "a-group", label: "A group", expanded: false }, children: [{ element: { id: "hidden", label: "Hidden", expanded: false } }, { element: { id: "match-a", label: "Match A", expanded: false } }] },
  ]);

  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["a-group", "match-a", "z-group", "match-b"]);
  assert.equal(model.getNode("hidden")?.visible, false);
  assert.equal(model.getNode("hidden")?.visibleChildIndex, -1);
  assert.equal(model.getNode("hidden")?.visibleChildrenCount, 1);
  assert.equal(model.getNode("match-a")?.visibleChildIndex, 0);
  assert.equal(model.getNode("match-a")?.visibleChildrenCount, 1);
  model.setFilter(undefined);
  assert.deepEqual(model.visibleNodes.map((node) => node.id), ["a-group", "hidden", "match-a", "z-group", "match-b"]);
  model.dispose();
});

test("ObjectTree projects model collapse and activation", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tree = new ObjectTree<TestNode>({
    ownerDocument: dom.window.document,
    ariaLabel: "Object tree",
    modelOptions: { identityProvider: { getId: (node) => node.id } },
    renderElement: (element) => {
      const label = dom.window.document.createElement("span");
      label.textContent = element.label;
      return label;
    },
  });
  const activated: string[] = [];
  tree.onDidActivate(({ element }) => activated.push(element.id));
  tree.setChildren([{
    element: { id: "parent", label: "Parent", expanded: false },
    collapsed: true,
    children: [{ element: { id: "child", label: "Child", expanded: false } }],
  }]);

  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 1);
  assert.equal(tree.expand("parent"), true);
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 2);
  tree.element.querySelector<HTMLButtonElement>("[data-tree-id='child']")?.click();
  assert.deepEqual(activated, ["child"]);
  tree.dispose();
  dom.window.close();
});

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
  assert.equal(parentRow.querySelectorAll(".zeta-tree-indent-guide").length, 0);

  parentRow.click();
  assert.deepEqual(activated, ["parent"]);
  parent.expanded = true;
  tree.items = [parent, sibling];
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 3);
  assert.equal(tree.element.querySelector(".zeta-tree-group")?.getAttribute("role"), "group");
  assert.equal(tree.element.querySelector("[data-tree-id='child']")?.getAttribute("aria-level"), "2");
  assert.equal(tree.element.querySelector("[data-tree-id='child']")?.querySelectorAll(".zeta-tree-indent-guide").length, 1);
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
