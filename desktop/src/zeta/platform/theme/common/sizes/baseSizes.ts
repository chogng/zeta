import { registerSize, size } from "../sizeRegistry.js";

const owner = "platform.ui";
const dimension = (id: string, value: number, description: string): string => registerSize(id, size(value), { description, owner });

export const actionBarGap = dimension("actionBar.gap", 2, "Gap between action bar items.");
export const toolbarItemGap = dimension("toolbar.itemGap", 2, "Gap before a toolbar item.");
export const toolbarActionSize = dimension("toolbar.actionSize", 24, "Default square toolbar action size.");
export const scrollbarSize = dimension("scrollbar.size", 10, "Default scrollbar thickness.");
export const sashDragAreaSize = dimension("sash.dragAreaSize", 4, "Default pointer target size for draggable Sash separators.");
export const sashHoverFeedbackSize = dimension("sash.hoverFeedbackSize", 1, "Default visible feedback size for hovered Sash separators.");
export const modalEditorWidth = dimension("modalEditor.width", 960, "Preferred modal editor width.");
export const modalEditorHeight = dimension("modalEditor.height", 720, "Preferred modal editor height.");
