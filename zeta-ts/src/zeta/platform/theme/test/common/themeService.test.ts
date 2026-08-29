import { strict as assert } from "node:assert";
import test from "node:test";
import {
	ColorId,
	colorCssVariable,
	colorIdentifiers,
	darkColorTheme,
	lightColorTheme,
} from "../../common/colorTheme.js";
import { ThemeService } from "../../common/themeService.js";

test("ThemeService exposes its initial theme and emits actual changes", () => {
	using service = new ThemeService(darkColorTheme);
	const changes: string[] = [];
	using listener = service.onDidColorThemeChange((theme) => {
		changes.push(theme.id);
	});

	service.setColorTheme(darkColorTheme);
	service.setColorTheme(lightColorTheme);

	assert.equal(service.getColorTheme(), lightColorTheme);
	assert.deepEqual(changes, ["zeta-light"]);
});

test("built-in themes define every registered color", () => {
	for (const id of colorIdentifiers) {
		assert.equal(typeof darkColorTheme.colors[id], "string");
		assert.equal(typeof lightColorTheme.colors[id], "string");
		assert.equal(darkColorTheme.getColor(id)?.toString(), darkColorTheme.colors[id]);
		assert.equal(lightColorTheme.getColor(id)?.toString(), lightColorTheme.colors[id]);
	}
});

test("color identifiers map to stable CSS custom properties", () => {
	assert.equal(
		colorCssVariable(ColorId.primaryButtonHoverBackground),
		"--zeta-button-primary-hover-background",
	);
	assert.equal(
		colorCssVariable(ColorId.titleBarForeground),
		"--zeta-title-bar-foreground",
	);
	assert.equal(
		colorCssVariable(ColorId.editorMultiCursorSecondaryBackground),
		"--zeta-editor-multi-cursor-secondary-background",
	);
});
