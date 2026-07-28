export function commandActionLabel(title) {
    return typeof title === "string" ? title : title.value;
}
export function isCommandActionToggleInfo(value) {
    return "condition" in value;
}
