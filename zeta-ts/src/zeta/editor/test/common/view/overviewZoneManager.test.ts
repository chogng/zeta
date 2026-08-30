import assert from 'node:assert/strict';
import test from 'node:test';
import { OverviewRulerZone, OverviewZoneManager } from '../../../common/viewModel/overviewZoneManager.js';

test('OverviewZoneManager maps line intervals to minimum visible pixel zones', () => {
	const manager = new OverviewZoneManager(lineNumber => (lineNumber - 1) * 20);
	const first = new OverviewRulerZone(1, 1, 0, 'warning');
	const last = new OverviewRulerZone(100, 100, 0, 'error');
	manager.setLineHeight(20);
	manager.setDOMWidth(6);
	manager.setDOMHeight(100);
	manager.setOuterHeight(2_000);
	manager.setZones([last, first]);

	const zones = manager.resolveColorZones();
	assert.equal(zones.length, 2);
	assert.deepEqual({ from: first.getColorZones()?.from, to: first.getColorZones()?.to }, { from: 0, to: 4 });
	assert.deepEqual({ from: last.getColorZones()?.from, to: last.getColorZones()?.to }, { from: 96, to: 100 });
	assert.equal(manager.getId2Color()[first.getColorZones()!.colorId], 'warning');
});

test('OverviewZoneManager invalidates cached geometry when the ruler changes size', () => {
	const manager = new OverviewZoneManager(lineNumber => (lineNumber - 1) * 10);
	const zone = new OverviewRulerZone(5, 6, 0, 'selection');
	manager.setLineHeight(10);
	manager.setDOMHeight(100);
	manager.setOuterHeight(100);
	manager.setZones([zone]);
	manager.resolveColorZones();
	const first = zone.getColorZones();
	manager.setDOMHeight(200);
	manager.resolveColorZones();
	const second = zone.getColorZones();
	assert.notEqual(second, first);
	assert.deepEqual({ from: second?.from, to: second?.to }, { from: 80, to: 120 });
});
