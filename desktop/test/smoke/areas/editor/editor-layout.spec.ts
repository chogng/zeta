import type { Locator } from "@playwright/test";
import { expect, test } from "../../../automation/test.js";

test("empty editor keeps its title, content, and watermark inside the editor part", async ({ workbench }) => {
  const editors = workbench.editors;
  const group = editors.groupAt(0);

  await expect(editors.element).toBeVisible();
  await expect(group.element).toBeVisible();
  await expect(group.title).toBeVisible();
  await expect(group.content).toBeVisible();
  await expect(group.watermark).toBeVisible();
  await expect(group.tabs).toHaveCount(0);

  await expect.poll(async () => editorGeometry(editors.element, group.element, group.title, group.content, group.watermark)).toEqual({
    groupFillsEditorClient: true,
    titleAboveContent: true,
    titleHasHeight: true,
    contentHasArea: true,
    watermarkInsideContent: true,
  });
});

test("editor layout remains valid across workbench window sizes", async ({ driver, workbench }) => {
  const editors = workbench.editors;
  const group = editors.groupAt(0);

  for (const size of [{ width: 900, height: 700 }, { width: 1200, height: 800 }, { width: 1494, height: 1104 }]) {
    await driver.setWindowSize(size);
    await expect.poll(async () => editorGeometry(editors.element, group.element, group.title, group.content, group.watermark), { message: `editor geometry at ${size.width}x${size.height}` }).toEqual({
      groupFillsEditorClient: true,
      titleAboveContent: true,
      titleHasHeight: true,
      contentHasArea: true,
      watermarkInsideContent: true,
    });
  }
});

async function editorGeometry(editor: Locator, group: Locator, title: Locator, content: Locator, watermark: Locator) {
  const [editorBox, editorClient, groupBox, titleBox, contentBox, watermarkBox] = await Promise.all([
    editor.boundingBox(),
    editor.evaluate(element => {
      const editorElement = element as HTMLElement;
      return {
        x: editorElement.clientLeft,
        y: editorElement.clientTop,
        width: editorElement.clientWidth,
        height: editorElement.clientHeight,
      };
    }),
    group.boundingBox(),
    title.boundingBox(),
    content.boundingBox(),
    watermark.boundingBox(),
  ]);
  if (!editorBox || !groupBox || !titleBox || !contentBox || !watermarkBox) {
    return null;
  }
  const tolerance = 1;
  return {
    groupFillsEditorClient: approximatelyEqual(groupBox.x, editorBox.x + editorClient.x, tolerance)
      && approximatelyEqual(groupBox.y, editorBox.y + editorClient.y, tolerance)
      && approximatelyEqual(groupBox.width, editorClient.width, tolerance)
      && approximatelyEqual(groupBox.height, editorClient.height, tolerance),
    titleAboveContent: titleBox.y + titleBox.height <= contentBox.y + tolerance,
    titleHasHeight: titleBox.height > 0,
    contentHasArea: contentBox.width > 0 && contentBox.height > 0,
    watermarkInsideContent: contains(contentBox, watermarkBox, tolerance),
  };
}

function approximatelyEqual(first: number, second: number, tolerance: number): boolean {
  return Math.abs(first - second) <= tolerance;
}

function contains(container: { x: number; y: number; width: number; height: number }, child: { x: number; y: number; width: number; height: number }, tolerance: number): boolean {
  return child.x >= container.x - tolerance
    && child.y >= container.y - tolerance
    && child.x + child.width <= container.x + container.width + tolerance
    && child.y + child.height <= container.y + container.height + tolerance;
}
