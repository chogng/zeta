import { strict as assert } from "node:assert";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import test from "node:test";

const sharedInteractionSelector = /\.zeta-(?:action-bar|button|tab(?:\b|-)|view-pane(?:\b|-))/;
const ariaStateSelector = /\[aria-(?:checked|pressed|selected)\b/;
const actionIdentitySelector = /\[data-action-id(?:\b|=)/;
const negatedProjectedStateSelector = /:not\(\.(?:active|checked|selected)\)/;

test("Workbench Part CSS does not reach into shared interaction controls", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await partCssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (sharedInteractionSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("CSS uses state classes instead of ARIA attributes as visual selectors", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (ariaStateSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("CSS state precedence does not negate projected state classes", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (negatedProjectedStateSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("CSS does not style action identity attributes", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (actionIdentitySelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("Workbench owns the horizontal ActionBar hover skin", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const actionBarCss = await readFile(join(sourceRoot, "base", "browser", "ui", "actionbar", "actionbar.css"), "utf8");
  const workbenchCss = await readFile(join(sourceRoot, "workbench", "browser", "media", "style.css"), "utf8");

  assert.doesNotMatch(actionBarCss, /:hover/);
  assert.match(workbenchCss, /\.zeta-workbench :where\(\.zeta-action-bar:not\(\.vertical\).*\.zeta-button:not\(:disabled\):hover\)/);
  assert.match(workbenchCss, /\.zeta-workbench :where\(\.zeta-action-bar:not\(\.vertical\).*\.zeta-action-label:not\(:disabled\):hover\)/);
  assert.match(workbenchCss, /background: var\(--zeta-toolbar-hover-background\)/);
});

test("ToolBar icon actions use the 22px borderless VS Code geometry", async () => {
  const sizeSource = await readFile(join(process.cwd(), "src", "zeta", "platform", "theme", "common", "sizes", "baseSizes.ts"), "utf8");
  const toolbarCss = await readFile(join(process.cwd(), "src", "zeta", "base", "browser", "ui", "toolbar", "toolbar.css"), "utf8");
  const tabListCss = await readFile(join(process.cwd(), "src", "zeta", "base", "browser", "ui", "tablist", "tablist.css"), "utf8");
  const compositeBarCss = await readFile(join(process.cwd(), "src", "zeta", "workbench", "browser", "parts", "compositebar", "compositebar.css"), "utf8");

  assert.match(sizeSource, /dimension\("toolbar\.actionSize", 22,/);
  assert.match(
    toolbarCss,
    /\.zeta-toolbar \.zeta-action-view-item\.icon > \.zeta-button \{[^}]*width: var\(--zeta-toolbar-action-size\);[^}]*border: 0;[^}]*padding: 3px;/s,
  );
  assert.match(tabListCss, /\.zeta-tab-actions \.zeta-action-view-item\.icon \.zeta-button \{[^}]*width: var\(--zeta-toolbar-action-size\);[^}]*padding: 3px;[^}]*border: 0;/s);
  assert.match(compositeBarCss, /\.zeta-composite-bar-overflow > \.zeta-button \{[^}]*width: var\(--zeta-toolbar-action-size\);[^}]*height: var\(--zeta-toolbar-action-size\);[^}]*padding: 3px;[^}]*border: 0;/s);
});

test("TabList owns a stable pointer cursor across labels and action gaps", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const tabListCss = await readFile(join(sourceRoot, "base", "browser", "ui", "tablist", "tablist.css"), "utf8");
  const compositeBarCss = await readFile(join(sourceRoot, "workbench", "browser", "parts", "compositebar", "compositebar.css"), "utf8");
  const editorTabsCss = await readFile(join(sourceRoot, "workbench", "browser", "parts", "editor", "media", "multiEditorTabsControl.css"), "utf8");

  assert.match(tabListCss, /\.zeta-tab\s*\{[^}]*cursor: pointer;/s);
  assert.match(tabListCss, /\.zeta-tab\.zeta-dnd-draggable\s*\{[^}]*cursor: pointer;/s);
  assert.match(tabListCss, /\.zeta-tab\.zeta-dnd-draggable:active\s*\{[^}]*cursor: grabbing;/s);
  assert.match(tabListCss, /\.zeta-tab-label\s*\{[^}]*cursor: inherit;/s);
  assert.doesNotMatch(compositeBarCss, /\.zeta-composite-bar \.zeta-tab-label\s*\{[^}]*cursor:/s);
  assert.doesNotMatch(editorTabsCss, /\.zeta-multi-editor-tabs-control \.zeta-tab-label\s*\{[^}]*cursor:/s);
});

test("TabList preserves the standard close-action hover background", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const tabListCss = await readFile(join(sourceRoot, "base", "browser", "ui", "tablist", "tablist.css"), "utf8");
  const editorTabsCss = await readFile(join(sourceRoot, "workbench", "browser", "parts", "editor", "media", "multiEditorTabsControl.css"), "utf8");
  const chatTabsCss = await readFile(join(sourceRoot, "workbench", "contrib", "chat", "browser", "view", "multiChatTabsControl.css"), "utf8");

  assert.match(tabListCss, /\.zeta-tab-actions \.zeta-action-view-item\.icon \.zeta-button:hover\s*\{[^}]*background: var\(--zeta-toolbar-hover-background\);/s);
  assert.doesNotMatch(editorTabsCss, /--zeta-tab-list-(?:checked-)?action-hover-background/);
  assert.doesNotMatch(chatTabsCss, /--zeta-tab-list-(?:checked-)?action-hover-background/);
});

test("Menubar icon actions hide only their text label", async () => {
  const menubarCss = await readFile(join(process.cwd(), "src", "zeta", "workbench", "browser", "parts", "titlebar", "menubarControl.css"), "utf8");

  assert.doesNotMatch(menubarCss, /\.zeta-menubar \.zeta-menubar-item > span\s*\{[^}]*display:\s*none/s);
  assert.match(menubarCss, /\.zeta-menubar \.zeta-menubar-item \.zeta-button-label\s*\{[^}]*display:\s*none/s);
});

test("Split actions own their joined geometry outside Terminal", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const dropdownCss = await readFile(join(sourceRoot, "base", "browser", "ui", "dropdown", "dropdown.css"), "utf8");
  const terminalCss = await readFile(join(sourceRoot, "workbench", "contrib", "terminal", "browser", "view", "media", "terminal.css"), "utf8");
  const workbenchCss = await readFile(join(sourceRoot, "workbench", "browser", "media", "style.css"), "utf8");

  assert.match(dropdownCss, /\.zeta-dropdown-with-primary-action-view-item\s*\{[^}]*display: flex;[^}]*gap: 0;[^}]*border-radius: 4px;/s);
  assert.match(workbenchCss, /\.zeta-dropdown-with-primary-action-view-item:not\(\.disabled\):hover/);
  assert.doesNotMatch(terminalCss, /zeta-terminal-(?:new|profile)-action/);
  assert.doesNotMatch(terminalCss, /margin-right:\s*-2px/);
});

async function partCssFiles(directory: string): Promise<string[]> {
  return (await cssFiles(directory)).filter((file) => /part\.css$/i.test(file));
}

async function cssFiles(directory: string): Promise<string[]> {
  const result: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await cssFiles(path));
    else if (entry.name.endsWith(".css")) result.push(path);
  }
  return result;
}
