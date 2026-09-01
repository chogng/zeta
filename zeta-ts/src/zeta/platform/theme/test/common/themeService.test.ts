import { strict as assert } from "node:assert";
import test from "node:test";
import {
	ColorId,
	colorCssVariable,
	colorIdentifiers,
	darkColorTheme,
	highContrastDarkColorTheme,
	highContrastLightColorTheme,
	lightColorTheme,
	sizeCssVariable,
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
	assert.equal(colorCssVariable(ColorId.editorFoldBackground), '--zeta-editor-fold-background');
	assert.equal(colorCssVariable(ColorId.editorFoldPlaceholderForeground), '--zeta-editor-fold-placeholder-foreground');
	assert.equal(colorCssVariable(ColorId.editorGutterFoldingControlForeground), '--zeta-editor-gutter-folding-control-foreground');
	assert.equal(colorCssVariable(ColorId.editorLineHighlightBackground), '--zeta-editor-line-highlight-background');
	assert.equal(colorCssVariable(ColorId.editorInactiveLineHighlightBackground), '--zeta-editor-inactive-line-highlight-background');
	assert.equal(colorCssVariable(ColorId.editorLineHighlightBorder), '--zeta-editor-line-highlight-border');
	assert.equal(sizeCssVariable('strokeThickness'), '--zeta-stroke-thickness');
});

test('current-line colors preserve transparent fills and high-contrast borders', () => {
	assert.deepEqual({
		darkBackground: darkColorTheme.colors[ColorId.editorLineHighlightBackground],
		lightBackground: lightColorTheme.colors[ColorId.editorLineHighlightBackground],
		highContrastDarkBorder: highContrastDarkColorTheme.colors[ColorId.editorLineHighlightBorder],
		highContrastLightBorder: highContrastLightColorTheme.colors[ColorId.editorLineHighlightBorder],
		strokeThickness: darkColorTheme.getSize('strokeThickness'),
	}, {
		darkBackground: 'rgba(0, 0, 0, 0)',
		lightBackground: 'rgba(0, 0, 0, 0)',
		highContrastDarkBorder: '#f38518',
		highContrastLightBorder: '#0f4a85',
		strokeThickness: { value: 1, unit: 'px' },
	});
});
