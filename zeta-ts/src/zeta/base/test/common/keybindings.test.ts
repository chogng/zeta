import assert from "node:assert/strict";
import test from "node:test";
import {
	getKeybindingLabel,
	KeybindingLabelStyle,
} from "../../common/keybindingLabels.js";
import { parseKeybinding } from "../../common/keybindingParser.js";
import {
	Keybinding,
	KeybindingChordKind,
	logicalKey,
	matchesResolvedChord,
	physicalKey,
	resolveKeybinding,
} from "../../common/keybindings.js";
import { OperatingSystem } from "../../common/platform.js";

test("keybinding parser distinguishes logical and physical chords", () => {
	const parsed = parseKeybinding("ctrl+k shift+[KeyP]");

	assert.ok(parsed);
	assert.equal(parsed.chords.length, 2);
	assert.equal(parsed.chords[0].kind, KeybindingChordKind.Logical);
	assert.equal(parsed.chords[1].kind, KeybindingChordKind.Physical);
	assert.equal(
		parsed.chords[1].kind === KeybindingChordKind.Physical
			? parsed.chords[1].code
			: undefined,
		"KeyP",
	);
	assert.equal(parseKeybinding("ctrl+shift"), undefined);
	assert.equal(parseKeybinding("primary+ctrl+k"), undefined);
	assert.equal(parseKeybinding("ctrl+k+v"), undefined);
});

test("portable primary modifiers resolve and format for each OS", () => {
	const keybinding = Keybinding.single(logicalKey("n", {
		primaryKey: true,
	}));
	const windows = resolveKeybinding(
		keybinding,
		OperatingSystem.Windows,
	);
	const mac = resolveKeybinding(
		keybinding,
		OperatingSystem.Macintosh,
	);

	assert.equal(getKeybindingLabel(windows), "Ctrl+N");
	assert.equal(getKeybindingLabel(mac), "⌘N");
	assert.equal(
		getKeybindingLabel(mac, KeybindingLabelStyle.Aria),
		"Command+N",
	);
	assert.equal(
		getKeybindingLabel(mac, KeybindingLabelStyle.UserSettings),
		"cmd+n",
	);
});

test("resolved chords match their declared logical or physical identity", () => {
	const resolved = resolveKeybinding(
		Keybinding.chord(
			logicalKey("z", { ctrlKey: true }),
			physicalKey("KeyY", { ctrlKey: true }),
		),
		OperatingSystem.Windows,
	);

	assert.equal(matchesResolvedChord(resolved.chords[0], {
		key: "Z",
		code: "KeyY",
		ctrlKey: true,
		shiftKey: false,
		altKey: false,
		metaKey: false,
	}), true);
	assert.equal(matchesResolvedChord(resolved.chords[1], {
		key: "z",
		code: "KeyY",
		ctrlKey: true,
		shiftKey: false,
		altKey: false,
		metaKey: false,
	}), true);
});

test("space has a stable user representation", () => {
	const parsed = parseKeybinding("ctrl+space");
	assert.ok(parsed);
	assert.equal(
		getKeybindingLabel(
			resolveKeybinding(parsed, OperatingSystem.Windows),
		),
		"Ctrl+Space",
	);
});
