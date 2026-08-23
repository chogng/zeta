import assert from "node:assert/strict";
import test from "node:test";
import { createServiceIdentifier, ServiceCollection } from "../../../platform/instantiation/common/instantiation.js";
import { WorkbenchServiceContributionRegistry, type WorkbenchServiceContributionContext } from "../../browser/workbenchServiceContributions.js";

test("Workbench service contributions install by declared dependency topology", () => {
	const first = createServiceIdentifier<{ readonly value: string }>("testFirst");
	const second = createServiceIdentifier<{ readonly value: string }>("testSecond");
	const registry = new WorkbenchServiceContributionRegistry();
	const installed: string[] = [];
	registry.register({ service: second, dependencies: [first], install: context => { installed.push("second"); return { value: `${context.services.get(first).value}:second` }; } });
	registry.register({ service: first, dependencies: [], install: () => { installed.push("first"); return { value: "first" }; } });
	const services = new ServiceCollection();
	registry.install(context(services));
	assert.deepEqual(installed, ["first", "second"]);
	assert.equal(services.get(second).value, "first:second");
});

test("Workbench service contributions reject duplicate and unresolved ownership", () => {
	const first = createServiceIdentifier<object>("testDuplicate");
	const missing = createServiceIdentifier<object>("testMissing");
	const registry = new WorkbenchServiceContributionRegistry();
	registry.register({ service: first, dependencies: [missing], install: () => ({}) });
	assert.throws(() => registry.register({ service: first, dependencies: [], install: () => ({}) }), /more than once/u);
	assert.throws(() => registry.install(context(new ServiceCollection())), /missing or cyclic dependencies.*testDuplicate.*testMissing/u);
});

function context(services: ServiceCollection): WorkbenchServiceContributionContext {
	return { services, own: value => value, blockRestorationUntil: () => undefined };
}
