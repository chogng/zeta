export const NATIVE_CONTEXT_MENU_POPUP_CHANNEL = "zeta:context-menu:popup";
export const NATIVE_CONTEXT_MENU_CLOSE_CHANNEL = "zeta:context-menu:close";
const MAX_MENU_DEPTH = 8;
const MAX_MENU_ITEMS = 256;
const MAX_ITEMS_PER_LEVEL = 64;
const MAX_ID_LENGTH = 128;
const MAX_LABEL_LENGTH = 512;
const MAX_COORDINATE = 1_000_000;
/** Validates the complete renderer-to-main native menu payload. */
export function validateNativeContextMenuRequest(value) {
    const request = exactRecord(value, ["items", "x", "y"]);
    const state = {
        itemCount: 0,
        ids: new Set(),
    };
    return {
        items: validateItems(request.items, 0, state),
        x: coordinate(request.x, "x"),
        y: coordinate(request.y, "y"),
    };
}
export function validateNativeContextMenuClose(value) {
    if (value === undefined)
        return {};
    exactRecord(value, []);
    return {};
}
function validateItems(value, depth, state) {
    if (!Array.isArray(value) || value.length === 0) {
        throw new Error("context menu items must be a non-empty array");
    }
    if (depth > MAX_MENU_DEPTH) {
        throw new Error("context menu nesting is too deep");
    }
    if (value.length > MAX_ITEMS_PER_LEVEL) {
        throw new Error("context menu level contains too many items");
    }
    return value.map((candidate, index) => {
        state.itemCount += 1;
        if (state.itemCount > MAX_MENU_ITEMS) {
            throw new Error("context menu contains too many items");
        }
        const item = looseRecord(candidate);
        switch (item.type) {
            case "separator":
                requireExactKeys(item, ["type"]);
                return { type: "separator" };
            case "action": {
                const keys = ["enabled", "id", "label", "type"];
                if (item.checked !== undefined)
                    keys.push("checked");
                if (item.accelerator !== undefined)
                    keys.push("accelerator");
                requireExactKeys(item, keys);
                const id = boundedString(item.id, `items[${index}].id`, MAX_ID_LENGTH);
                if (state.ids.has(id)) {
                    throw new Error(`duplicate context menu action id: ${id}`);
                }
                state.ids.add(id);
                const result = {
                    type: "action",
                    id,
                    label: boundedString(item.label, `items[${index}].label`, MAX_LABEL_LENGTH),
                    enabled: boolean(item.enabled, `items[${index}].enabled`),
                    ...(item.accelerator === undefined
                        ? {}
                        : {
                            accelerator: boundedString(item.accelerator, `items[${index}].accelerator`, MAX_ID_LENGTH),
                        }),
                };
                return item.checked === undefined
                    ? result
                    : {
                        ...result,
                        checked: boolean(item.checked, `items[${index}].checked`),
                    };
            }
            case "submenu":
                requireExactKeys(item, [
                    "enabled",
                    "items",
                    "label",
                    "type",
                ]);
                return {
                    type: "submenu",
                    label: boundedString(item.label, `items[${index}].label`, MAX_LABEL_LENGTH),
                    enabled: boolean(item.enabled, `items[${index}].enabled`),
                    items: validateItems(item.items, depth + 1, state),
                };
            default:
                throw new Error(`items[${index}].type is invalid`);
        }
    });
}
function exactRecord(value, keys) {
    const result = looseRecord(value);
    requireExactKeys(result, keys);
    return result;
}
function looseRecord(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error("context menu payload must contain objects");
    }
    return value;
}
function requireExactKeys(value, keys) {
    const actual = Object.keys(value).sort();
    const expected = [...keys].sort();
    if (actual.length !== expected.length ||
        actual.some((key, index) => key !== expected[index])) {
        throw new Error(`context menu object must contain exactly: ${expected.join(", ")}`);
    }
}
function boundedString(value, field, maxLength) {
    if (typeof value !== "string" ||
        value.trim().length === 0 ||
        value.length > maxLength) {
        throw new Error(`${field} must be a non-empty bounded string`);
    }
    return value;
}
function boolean(value, field) {
    if (typeof value !== "boolean") {
        throw new Error(`${field} must be a boolean`);
    }
    return value;
}
function coordinate(value, field) {
    if (!Number.isSafeInteger(value) ||
        Math.abs(value) > MAX_COORDINATE) {
        throw new Error(`${field} must be a bounded safe integer`);
    }
    return value;
}
