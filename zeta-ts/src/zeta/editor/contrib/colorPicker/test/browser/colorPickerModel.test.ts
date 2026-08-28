import assert from 'node:assert/strict';
import test from 'node:test';
import { TextPosition, TextRange } from '../../../../common/core/text.js';
import { RGBA8 } from '../../../../common/core/misc/rgba.js';
import { ColorPickerModel } from '../../browser/colorPickerModel.js';

test('color picker model retains format choice while provider presentations refresh', () => {
	using model = new ColorPickerModel(new RGBA8(255, 0, 0, 128));
	const range = TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 9));
	const presentations = ['rgba(255, 0, 0, 0.5)', 'hsla(0, 100%, 50%, 0.5)', '#ff000080'].map(label => ({ label, textEdit: { range, text: label } }));

	model.setColorPresentations(presentations, '#ff000080');
	model.setColor(new RGBA8(0, 255, 0, 128));
	model.setColorPresentations([
		{ label: 'rgba(0, 255, 0, 0.5)', textEdit: { range, text: 'rgba(0, 255, 0, 0.5)' } },
		{ label: 'hsla(120, 100%, 50%, 0.5)', textEdit: { range, text: 'hsla(120, 100%, 50%, 0.5)' } },
		{ label: '#00ff0080', textEdit: { range, text: '#00ff0080' } },
	]);

	assert.equal(model.selectedPresentation?.label, '#00ff0080');
	model.selectNextPresentation();
	assert.equal(model.selectedPresentation?.label, 'rgba(0, 255, 0, 0.5)');
});
