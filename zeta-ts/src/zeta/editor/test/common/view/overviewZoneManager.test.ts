import assert from 'node:assert/strict';
import test from 'node:test';
import { OverviewRulerZone, OverviewZoneManager } from '../../../common/viewModel/overviewZoneManager.js';

test('OverviewZoneManager maps line intervals to minimum visible pixel zones', () => {
	const manager = new OverviewZoneManager(lineIndex => lineIndex * 20);
	const first = new OverviewRulerZone(0, 1, 0, 'warning');
	const last = new OverviewRulerZone(99, 100, 0, 'error');
	manager.setLineHeight(20);
	manager.setDOMWidth(6);
	manager.setDOMHeight(100);
	manager.setOuterHeight(2_000);
	manager.setZones([last, first]);

	const zones = manager.resolveColorZones();
	assert.equal(zones.length, 2);
	assert.deepEqual({ from: first.getColorZone()?.from, to: first.getColorZone()?.to }, { from: 0, to: 4 });
	assert.deepEqual({ from: last.getColorZone()?.from, to: last.getColorZone()?.to }, { from: 96, to: 100 });
	assert.equal(manager.getIdToColor()[first.getColorZone()!.colorId], 'warning');
});

test('OverviewZoneManager invalidates cached geometry when the ruler changes size', () => {
	const manager = new OverviewZoneManager(lineIndex => lineIndex * 10);
	const zone = new OverviewRulerZone(5, 6, 0, 'selection');
	manager.setLineHeight(10);
	manager.setDOMHeight(100);
	manager.setOuterHeight(100);
	manager.setZones([zone]);
	manager.resolveColorZones();
	const first = zone.getColorZone();
	manager.setDOMHeight(200);
	manager.resolveColorZones();
	const second = zone.getColorZone();
	assert.notEqual(second, first);
	assert.deepEqual({ from: second?.from, to: second?.to }, { from: 100, to: 120 });
});
