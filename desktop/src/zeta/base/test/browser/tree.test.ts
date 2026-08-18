import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AsyncDataTree } from "../../browser/ui/tree/asyncDataTree.js";
import { DataTree } from "../../browser/ui/tree/dataTree.js";
import { IndexTree } from "../../browser/ui/tree/indexTree.js";
import { IndexTreeModel } from "../../browser/ui/tree/indexTreeModel.js";
import { ObjectTree } from "../../browser/ui/tree/objectTree.js";
import { ObjectTreeModel } from "../../browser/ui/tree/objectTreeModel.js";
import { TreeVisibility } from "../../browser/ui/tree/tree.js";

interface TestNode {
  readonly id: string;
  readonly label: string;
  expanded: boolean;
  readonly children?: readonly TestNode[];
  readonly collapsible?: boolean;
  readonly collapsed?: boolean;
}

test("IndexTreeModel owns index locations and atomic splice", () => {
  const model = new IndexTreeModel<TestNode>({ id: "root", label: "Root", expanded: true });
  model.setChildren([
    { element: { id: "parent", label: "Parent", expanded: false }, children: [{ element: { id: "child", label: "Child", expanded: false } }] },
    { element: { id: "sibling", label: "Sibling", expanded: false } },
  ]);
  assert.equal(model.getNode([0]).element.id, "parent");
  assert.equal(model.getNode([0, 0]).element.id, "child");
  assert.deepEqual(model.getNode([0, 0]).location, [0, 0]);
  model.splice([1], 0, [{ element: { id: "inserted", label: "Inserted", expanded: false } }]);
  assert.deepEqual(model.rootNodes.map((node) => node.element.id), ["parent", "inserted", "sibling"]);
  assert.deepEqual(model.getNode([2]).location, [2]);
  assert.equal(model.collapse([0]), true);
  assert.deepEqual(model.visibleNodes.map((node) => node.element.id), ["parent", "inserted", "sibling"]);
  assert.throws(() => model.splice([], 0), /child position/);
  assert.equal(model.getNode([2]).element.id, "sibling");
  model.dispose();
});

test("IndexTree renders splice results through the shared flat AbstractTree", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tree = new IndexTree<TestNode>({ id: "root", label: "Root", expanded: true }, {
    ownerDocument: dom.window.document,
    ariaLabel: "Index tree",
    renderElement: (element) => {
      const label = dom.window.document.createElement("span");
      label.textContent = element.label;
      return label;
    },
  });
  tree.splice([0], 0, [{ element: { id: "first", label: "First", expanded: false } }]);
  tree.splice([1], 0, [{ element: { id: "second", label: "Second", expanded: false } }]);
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(":scope > .zeta-tree-row")].map((row) => row.textContent), ["First", "Second"]);
  assert.equal(tree.element.querySelector(".zeta-tree-row")?.getAttribute("aria-posinset"), "1");
  tree.dispose();
  dom.window.close();
});

test("DataTree materializes and refreshes a synchronous data source", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const root: TestNode = { id: "root", label: "Root", expanded: true };
  const group: TestNode = { id: "group", label: "Group", expanded: false };
  const first: TestNode = { id: "first", label: "First", expanded: false };
  const second: TestNode = { id: "second", label: "Second", expanded: false };
  const children = new Map<TestNode, readonly TestNode[]>([[root, [group]], [group, [first]], [first, []], [second, []]]);
  const tree = new DataTree<TestNode, TestNode>({
    hasChildren: (element) => (children.get(element as TestNode)?.length ?? 0) > 0,
    getChildren: (element) => children.get(element as TestNode) ?? [],
  }, {
    ownerDocument: dom.window.document,
    identityProvider: { getId: (element) => element.id },
    collapseByDefault: (element) => element === group,
    renderElement: (element) => {
      const label = dom.window.document.createElement("span");
      label.textContent = element.label;
      return label;
    },
  });
  tree.setInput(root);
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(".zeta-tree-row")].map((row) => row.textContent), ["Group"]);
  tree.expand(group);
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(".zeta-tree-row")].map((row) => row.textContent), ["Group", "First"]);
  children.set(group, [second]);
  tree.updateChildren(group);
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(".zeta-tree-row")].map((row) => row.textContent), ["Group", "Second"]);
  tree.dispose();
  dom.window.close();
});

test("AsyncDataTree loads on expansion and rejects stale refresh results", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const root: TestNode = { id: "root", label: "Root", expanded: true };
  const group: TestNode = { id: "group", label: "Group", expanded: false };
  const stale: TestNode = { id: "stale", label: "Stale", expanded: false };
  const current: TestNode = { id: "current", label: "Current", expanded: false };
  const pending: Array<(children: readonly TestNode[]) => void> = [];
  const tree = new AsyncDataTree<TestNode, TestNode>({
    hasChildren: (element) => element === root || element === group,
    getChildren: (element) => {
      if (element === root) return [group];
      return new Promise<readonly TestNode[]>((resolve) => pending.push(resolve));
    },
  }, {
    ownerDocument: dom.window.document,
    identityProvider: { getId: (element) => element.id },
    renderElement: (element) => {
      const label = dom.window.document.createElement("span");
      label.textContent = element.label;
      return label;
    },
  });
  await tree.setInput(root);
  assert.equal(tree.expand(group), true);
  assert.equal(pending.length, 1);
  const latest = tree.updateChildren(group);
  assert.equal(pending.length, 2);
  pending[1]!([current]);
  await latest;
  pending[0]!([stale]);
  await Promise.resolve();
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(".zeta-tree-row")].map((row) => row.textContent), ["Group", "Current"]);
  tree.dispose();
  dom.window.close();
});

test("ObjectTreeModel owns hierarchy, local replacement, and collapse state", () => {
  const child: TestNode = { id: "child", label: "Child", expanded: false };
  const parent: TestNode = { id: "parent", label: "Parent", expanded: false, children: [child] };
  const model = new ObjectTreeModel<TestNode>({ identityProvider: { getId: (node) => node.id } });
  const changes: string[] = [];
  const collapseChanges: string[] = [];
  model.onDidChange((event) => changes.push(event.kind));
  model.onDidChangeCollapseState(({ node, collapsed }) => collapseChanges.push(`${node.id}:${collapsed}`));

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
  assert.deepEqual(collapseChanges, ["parent:true"]);
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

test("ObjectTree projects the model as flat list rows with tree ARIA", () => {
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
  const pointers: string[] = [];
  const collapsed: string[] = [];
  tree.onDidActivate(({ element }) => activated.push(element.id));
  tree.onPointer(({ element, target }) => pointers.push(`${element.id}:${target}`));
  tree.onDidChangeCollapseState(({ element, collapsed: isCollapsed }) => collapsed.push(`${element.id}:${isCollapsed}`));
  tree.setChildren([{
    element: { id: "parent", label: "Parent", expanded: false },
    collapsed: true,
    children: [{ element: { id: "child", label: "Child", expanded: false } }],
  }, { element: { id: "sibling", label: "Sibling", expanded: false } }]);

  assert.equal(tree.element.tagName, "DIV");
  assert.equal(tree.element.getAttribute("role"), "tree");
  assert.equal(tree.element.getAttribute("aria-label"), "Object tree");
  assert.equal(tree.element.tabIndex, 0);
  assert.equal(tree.element.querySelectorAll(".zeta-tree-group, .zeta-tree-node").length, 0);
  assert.equal(tree.element.querySelectorAll(":scope > .zeta-tree-row").length, 2);
  const parent = tree.element.querySelector<HTMLElement>("[data-tree-id='parent']");
  const sibling = tree.element.querySelector<HTMLElement>("[data-tree-id='sibling']");
  assert.ok(parent);
  assert.ok(sibling);
  assert.equal(parent.getAttribute("role"), "treeitem");
  assert.equal(parent.getAttribute("aria-level"), "1");
  assert.equal(parent.getAttribute("aria-posinset"), "1");
  assert.equal(parent.getAttribute("aria-setsize"), "2");
  assert.equal(parent.getAttribute("aria-expanded"), "false");
  assert.equal(sibling.getAttribute("aria-posinset"), "2");
  assert.equal(tree.expand("parent"), true);
  assert.deepEqual(collapsed, ["parent:false"]);
  assert.deepEqual([...tree.element.querySelectorAll<HTMLElement>(":scope > .zeta-tree-row")].map((row) => row.dataset.treeId), ["parent", "child", "sibling"]);
  const child = tree.element.querySelector<HTMLElement>("[data-tree-id='child']");
  assert.equal(child?.getAttribute("aria-level"), "2");
  assert.equal(child?.getAttribute("aria-posinset"), "1");
  assert.equal(child?.getAttribute("aria-setsize"), "1");
  assert.equal(child?.querySelectorAll(".zeta-tree-indent-guide").length, 1);
  tree.element.querySelector<HTMLElement>("[data-tree-id='parent'] .zeta-tree-twistie")?.click();
  assert.deepEqual(collapsed, ["parent:false", "parent:true"]);
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 2);
  tree.expand("parent");
  assert.deepEqual(collapsed, ["parent:false", "parent:true", "parent:false"]);
  const parentRow = tree.element.querySelector<HTMLElement>("[data-tree-id='parent']");
  assert.ok(parentRow);
  const parentIcon = dom.window.document.createElementNS("http://www.w3.org/2000/svg", "svg");
  const parentIconPath = dom.window.document.createElementNS("http://www.w3.org/2000/svg", "path");
  parentIcon.append(parentIconPath);
  parentRow.querySelector(".zeta-tree-contents")?.prepend(parentIcon);
  parentIconPath.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  assert.deepEqual(activated, ["parent"]);
  assert.deepEqual(pointers, ["parent:contents"]);
  assert.deepEqual(collapsed, ["parent:false", "parent:true", "parent:false"]);
  assert.equal(tree.element.querySelectorAll(".zeta-tree-row").length, 3);

  tree.dispose();
  dom.window.close();
});

test("ObjectTree delegates focus, selection, and keyboard navigation to List", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tree = createObjectTree(dom);
  tree.setChildren([
    { element: { id: "first", label: "First", expanded: false } },
    { element: { id: "second", label: "Second", expanded: false } },
  ]);
  const focusChanges: string[] = [];
  const selectionChanges: string[] = [];
  const accepted: string[] = [];
  tree.onDidChangeFocus(({ element }) => focusChanges.push(element?.id ?? "none"));
  tree.onDidChangeSelection(({ elements }) => selectionChanges.push(elements.map((element) => element.id).join(",")));
  tree.onDidAccept(({ element }) => accepted.push(element.id));
  dom.window.document.body.append(tree.element);
  const firstRow = tree.element.querySelector<HTMLElement>("[data-tree-id='first']");
  const secondRow = tree.element.querySelector<HTMLElement>("[data-tree-id='second']");
  assert.ok(firstRow);
  assert.ok(secondRow);
  assert.equal(tree.element.tabIndex, 0);
  assert.equal(firstRow.tabIndex, -1);
  assert.equal(secondRow.tabIndex, -1);

  tree.element.focus();
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowDown" }));
  assert.equal(dom.window.document.activeElement, tree.element);
  assert.equal(tree.focus?.id, "second");
  assert.deepEqual(tree.selection.map((element) => element.id), ["second"]);
  assert.equal(tree.element.getAttribute("aria-activedescendant"), secondRow.id);
  assert.equal(secondRow.getAttribute("aria-selected"), "true");
  assert.ok(secondRow.classList.contains("selected"));

  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "Home" }));
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "Enter" }));
  assert.deepEqual(focusChanges, ["second", "first"]);
  assert.deepEqual(selectionChanges, ["second", "first"]);
  assert.deepEqual(accepted, ["first"]);

  tree.dispose();
  dom.window.close();
});

test("ObjectTree keyboard expansion operates on model nodes", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tree = createObjectTree(dom);
  tree.setChildren([{
    element: { id: "parent", label: "Parent", expanded: false },
    collapsed: true,
    children: [{ element: { id: "child", label: "Child", expanded: false } }],
  }]);
  const collapseChanges: string[] = [];
  tree.onDidChangeCollapseState(({ element, collapsed }) => collapseChanges.push(`${element.id}:${collapsed}`));
  tree.setFocus("parent");
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowRight" }));
  assert.deepEqual(tree.model.visibleNodes.map((node) => node.id), ["parent", "child"]);
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowRight" }));
  assert.equal(tree.focus?.id, "child");
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowLeft" }));
  assert.equal(tree.focus?.id, "parent");
  tree.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, key: "ArrowLeft" }));
  assert.deepEqual(tree.model.visibleNodes.map((node) => node.id), ["parent"]);
  assert.deepEqual(collapseChanges, ["parent:false", "parent:true"]);
  tree.dispose();
  dom.window.close();
});

function createObjectTree(dom: JSDOM): ObjectTree<TestNode> {
  return new ObjectTree<TestNode>({
    ownerDocument: dom.window.document,
    ariaLabel: "Test tree",
    indentGuides: "always",
    modelOptions: { identityProvider: { getId: (node) => node.id } },
    renderElement: (element) => {
      const label = dom.window.document.createElement("span");
      label.textContent = element.label;
      return label;
    },
  });
}
