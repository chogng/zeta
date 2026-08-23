import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../../base/common/event.js";
import { ContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import { type IConfigurationChangeEvent, type IConfigurationKey, type IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { AccessibilityConfiguration, AccessibilitySupport, CONTEXT_ACCESSIBILITY_MODE_ENABLED } from "../../../../platform/accessibility/common/accessibility.js";
import { AccessibilityService } from "../../../../platform/accessibility/browser/accessibilityService.js";
import { h } from "../../../../base/browser/dom.js";

class TestConfigurationService implements IConfigurationService {
	private readonly changeEmitter = new Emitter<IConfigurationChangeEvent>();
	private readonly values = new Map<string, unknown>();

	readonly onDidChangeConfiguration = this.changeEmitter.event;

	getValue<T>(key: IConfigurationKey<T>): T {
		return (this.values.get(key.key) ?? key.defaultValue) as T;
	}

	async updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
		this.values.set(key.key, key.parse(key.serialize(value)));
		this.changeEmitter.fire({
			keys: new Set([key.key]),
			affectsConfiguration(candidate) {
				return candidate.key === key.key;
			},
		});
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		this.values.delete(key.key);
		this.changeEmitter.fire({
			keys: new Set([key.key]),
			affectsConfiguration(candidate) {
				return candidate.key === key.key;
			},
		});
	}

	async reload(): Promise<void> {}

	dispose(): void {
		this.changeEmitter.dispose();
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

test("accessibility service projects screen-reader support and context state", () => {
	const environment = new JSDOM("<!doctype html><html><body></body></html>", { pretendToBeVisual: true });
	const root = h(environment.window.document, "main");
	environment.window.document.body.append(root);
	using configuration = new TestConfigurationService();
	using contextKeys = new ContextKeyService();
	using service = new AccessibilityService({
		root,
		contextKeyService: contextKeys,
		configurationService: configuration,
	});

	assert.equal(service.getAccessibilitySupport(), AccessibilitySupport.Unknown);
	assert.equal(service.isScreenReaderOptimized(), false);
	assert.equal(contextKeys.getValue(CONTEXT_ACCESSIBILITY_MODE_ENABLED.key), false);

	let changes = 0;
	using subscription = service.onDidChangeScreenReaderOptimized(() => changes += 1);
	service.setAccessibilitySupport(AccessibilitySupport.Enabled);
	assert.equal(service.isScreenReaderOptimized(), true);
	assert.equal(contextKeys.getValue(CONTEXT_ACCESSIBILITY_MODE_ENABLED.key), true);
	assert.equal(changes, 1);

	service.setAccessibilitySupport(AccessibilitySupport.Enabled);
	assert.equal(changes, 1);
});

test("accessibility service applies reduction and link presentation policies", async () => {
	const environment = new JSDOM("<!doctype html><html><body></body></html>", { pretendToBeVisual: true });
	const root = h(environment.window.document, "main");
	environment.window.document.body.append(root);
	using configuration = new TestConfigurationService();
	using contextKeys = new ContextKeyService();
	using service = new AccessibilityService({
		root,
		contextKeyService: contextKeys,
		configurationService: configuration,
	});

	assert.equal(root.classList.contains("zeta-enable-motion"), true);
	assert.equal(root.classList.contains("zeta-reduce-transparency"), false);
	assert.equal(root.classList.contains("zeta-underline-links"), false);

	await configuration.updateValue(AccessibilityConfiguration.reduceMotion, "on");
	await configuration.updateValue(AccessibilityConfiguration.reduceTransparency, "on");
	await configuration.updateValue(AccessibilityConfiguration.underlineLinks, true);
	assert.equal(root.classList.contains("zeta-reduce-motion"), true);
	assert.equal(root.classList.contains("zeta-enable-motion"), false);
	assert.equal(root.classList.contains("zeta-reduce-transparency"), true);
	assert.equal(root.classList.contains("zeta-underline-links"), true);

	service.status("Saved");
	service.alert("Error");
	assert.equal(environment.window.document.querySelectorAll(".zeta-aria-live").length, 1);
});
