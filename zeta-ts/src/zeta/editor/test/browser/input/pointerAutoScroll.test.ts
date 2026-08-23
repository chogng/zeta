import assert from "node:assert/strict";
import test from "node:test";
import { getStanzaPointerAutoScrollVelocity } from "../../../browser/input/pointerAutoScroll.js";

const bounds = {
	left: 100,
	top: 50,
	right: 300,
	bottom: 150,
};

test("Pointer autoscroll velocity follows overflow on each axis", () => {
	assert.deepEqual(
		getStanzaPointerAutoScrollVelocity(bounds, {
			clientX: 200,
			clientY: 100,
		}),
		{ left: 0, top: 0 },
	);
	assert.deepEqual(
		getStanzaPointerAutoScrollVelocity(bounds, {
			clientX: 90,
			clientY: 170,
		}),
		{ left: -420, top: 600 },
	);
	assert.deepEqual(
		getStanzaPointerAutoScrollVelocity(bounds, {
			clientX: 300,
			clientY: 150,
		}),
		{ left: 240, top: 240 },
	);
	assert.deepEqual(
		getStanzaPointerAutoScrollVelocity(bounds, {
			clientX: 1_000,
			clientY: -1_000,
		}),
		{ left: 2_400, top: -2_400 },
	);
});

test("Pointer autoscroll velocity validates bounds and points", () => {
	assert.throws(() => getStanzaPointerAutoScrollVelocity(
		{ ...bounds, right: 50 },
		{ clientX: 100, clientY: 100 },
	), /bounds are invalid/);
	assert.throws(() => getStanzaPointerAutoScrollVelocity(
		bounds,
		{ clientX: Number.NaN, clientY: 100 },
	), /finite coordinates/);
});
