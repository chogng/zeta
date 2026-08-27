import assert from 'node:assert/strict';
import test from 'node:test';
import { darkColorTheme, highContrastDarkColorTheme, highContrastLightColorTheme, lightColorTheme } from '../../../platform/theme/common/colorTheme.js';
import { ColorScheme } from '../../../platform/theme/common/theme.js';
import { StandaloneThemeService } from '../../standalone/browser/standaloneThemeService.js';

class TestMediaQueryList extends EventTarget {
	public matches = false;

	public setMatches(matches: boolean): void {
		if (this.matches === matches) {
			return;
		}
		this.matches = matches;
		this.dispatchEvent(new Event('change'));
	}
}

function createThemeService(): { readonly mediaQuery: TestMediaQueryList; readonly service: StandaloneThemeService } {
	const mediaQuery = new TestMediaQueryList();
	const ownerWindow = {
		matchMedia(query: string): MediaQueryList {
			assert.equal(query, '(forced-colors: active)');
			return mediaQuery as unknown as MediaQueryList;
		},
	} as Window;
	return { mediaQuery, service: new StandaloneThemeService(ownerWindow) };
}

test('standalone themes default to the built-in light theme', () => {
	const fixture = createThemeService();
	using service = fixture.service;
	assert.equal(service.getColorTheme(), lightColorTheme);
});

test('standalone themes register by ID and refresh the active theme', () => {
	const fixture = createThemeService();
	using service = fixture.service;
	const events: string[] = [];
	using listener = service.onDidColorThemeChange(theme => events.push(theme.id));

	service.defineTheme('sample-dark', {
		label: 'Sample Dark',
		colorScheme: ColorScheme.Dark,
		colors: { 'editor.background': '#101010' },
	});
	service.setTheme('sample-dark');
	assert.equal(service.getColorTheme().getColorCss('editor.background'), '#101010');

	service.defineTheme('sample-dark', {
		label: 'Updated Sample Dark',
		colorScheme: ColorScheme.Dark,
		colors: { 'editor.background': '#202020' },
	});
	assert.deepEqual(events, ['sample-dark', 'sample-dark']);
	assert.equal(service.getColorTheme().getColorCss('editor.background'), '#202020');
	assert.throws(() => service.setTheme('missing-theme'), /Unknown standalone color theme/);
});

test('standalone themes track forced colors without losing the selected theme', () => {
	const fixture = createThemeService();
	using service = fixture.service;
	service.setTheme(darkColorTheme.id);
	fixture.mediaQuery.setMatches(true);
	assert.equal(service.getColorTheme(), highContrastDarkColorTheme);
	service.defineTheme(highContrastDarkColorTheme.id, {
		label: 'Updated High Contrast Dark',
		colorScheme: ColorScheme.HighContrastDark,
		colors: { 'editor.background': '#010101' },
	});
	assert.equal(service.getColorTheme().getColorCss('editor.background'), '#010101');

	fixture.mediaQuery.setMatches(false);
	assert.equal(service.getColorTheme(), darkColorTheme);
	service.setTheme(lightColorTheme.id);
	fixture.mediaQuery.setMatches(true);
	assert.equal(service.getColorTheme(), highContrastLightColorTheme);

	service.setAutoDetectHighContrast(false);
	assert.equal(service.getColorTheme(), lightColorTheme);
});
