import { strict as assert } from "node:assert";
import test from "node:test";
import {
  Icon,
  registerIcon,
  resolveIconDefinition,
} from "../../common/icon.js";

let testIconId = 0;

function nextIconId(suffix: string): string {
  testIconId += 1;
  return `test-icon-${testIconId}-${suffix}`;
}

test("registered icons resolve to their renderer definition", () => {
  const definition = () => "<svg></svg>";
  const icon = registerIcon(nextIconId("definition"), definition);

  assert.equal(resolveIconDefinition(icon), definition);
});

test("semantic icons resolve through another icon", () => {
  const definition = () => "<svg></svg>";
  const libraryIcon = registerIcon(nextIconId("library"), definition);
  const semanticIcon = registerIcon(nextIconId("semantic"), libraryIcon);

  assert.equal(resolveIconDefinition(semanticIcon), definition);
});

test("unknown icon IDs fail at the renderer boundary", () => {
  const id = nextIconId("unknown");

  assert.throws(
    () => resolveIconDefinition(Icon.fromId(id)),
    new ReferenceError(`Unknown icon '${id}'`),
  );
});

test("circular icon defaults report the complete alias chain", () => {
  const firstId = nextIconId("cycle-first");
  const secondId = nextIconId("cycle-second");
  const first = registerIcon(firstId, Icon.fromId(secondId));
  registerIcon(secondId, first);

  assert.throws(
    () => resolveIconDefinition(first),
    new Error(
      `Circular icon defaults: ${firstId} -> ${secondId} -> ${firstId}`,
    ),
  );
});

test("duplicate and malformed icon IDs are rejected", () => {
  const id = nextIconId("duplicate");
  const definition = () => "<svg></svg>";
  registerIcon(id, definition);

  assert.throws(() => registerIcon(id, definition), TypeError);
  assert.throws(() => registerIcon("Not Valid", definition), TypeError);
});
