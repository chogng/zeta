import { DisposableStore, toDisposable } from '../../../src/zeta/base/common/lifecycle.js';
import * as stanzaApi from '../../../src/zeta/editor/editor.main.js';
import '../../../src/zeta/editor/editor.code.all.js';

const initialText = `interface GeometrySample {
	readonly label: string;
	readonly columns: readonly number[];
}

const greeting = "你好，Stanza 👋";
const sample: GeometrySample = {
	label: greeting,
	columns: [0, 4, 8, 16, 32, 64, 80],
};

export function describe(sample: GeometrySample): string {
	const longLine = "Edit this deliberately long line to inspect wrapping, cursor placement, selections, horizontal geometry, and viewport updates without starting the Zeta Workbench.";
	return \`\${sample.label}: \${sample.columns.join(", ")} — \${longLine}\`;
}

console.log(describe(sample));
`;

interface GpuTextIntegrationHarness {
	readonly initialText: string;
	getValue(): string;
	resetGpuFrameTrace(): void;
	readGpuFrameTrace(): readonly GpuRenderPassTrace[];
	dispose(): void;
}

interface GpuRenderPassTrace {
	readonly label: string;
	readonly loadOp: GPULoadOp;
	readonly viewId: number;
	readonly submissionId: number;
}

interface GpuFrameTraceController {
	reset(): void;
	read(): readonly GpuRenderPassTrace[];
	dispose(): void;
}

declare global {
	interface Window {
		zetaGpuTextIntegration: GpuTextIntegrationHarness;
	}
}

const container = requiredElement('editor-root');
const disposables = new DisposableStore();
const gpuFrameTrace = installGpuFrameTrace();
disposables.add(toDisposable(() => gpuFrameTrace.dispose()));
const resource = stanzaApi.URI.parse('inmemory://stanza/gpu-integration.ts');
const model = disposables.add(stanzaApi.editor.createModel(initialText, 'typescript', resource));
const editor = disposables.add(stanzaApi.editor.create(container, {
	model,
	lineWrapping: stanzaApi.EditorLineWrapping.On,
	showLineNumbers: true,
	showIndentationGuides: true,
	bracketPairColorization: true,
	experimentalGpuAcceleration: 'on',
}));
const resizeObserver = new ResizeObserver(() => editor.layout({ width: container.clientWidth, height: container.clientHeight }));
resizeObserver.observe(container);
disposables.add(toDisposable(() => resizeObserver.disconnect()));
editor.layout({ width: container.clientWidth, height: container.clientHeight });
editor.focus();

window.zetaGpuTextIntegration = {
	initialText,
	getValue: () => editor.getValue(),
	resetGpuFrameTrace: () => gpuFrameTrace.reset(),
	readGpuFrameTrace: () => gpuFrameTrace.read(),
	dispose: () => disposables.dispose(),
};

function installGpuFrameTrace(): GpuFrameTraceController {
	const gpu = navigator.gpu;
	if (!gpu) throw new Error('GPU integration test requires WebGPU');
	const originalRequestAdapter = gpu.requestAdapter;
	let passes: GpuRenderPassTrace[] = [];
	let viewIds = new WeakMap<GPUTexture | GPUTextureView, number>();
	let nextViewId = 1;
	let nextSubmissionId = 1;
	const commandBufferPasses = new WeakMap<GPUCommandBuffer, Omit<GpuRenderPassTrace, 'submissionId'>[]>();
	gpu.requestAdapter = async options => {
		const adapter = await originalRequestAdapter.call(gpu, options);
		if (!adapter) return null;
		const originalRequestDevice = adapter.requestDevice;
		adapter.requestDevice = async descriptor => {
			const device = await originalRequestDevice.call(adapter, descriptor);
			const originalSubmit = device.queue.submit;
			device.queue.submit = commandBuffers => {
				const buffers = [...commandBuffers];
				const submissionId = nextSubmissionId;
				nextSubmissionId += 1;
				for (const commandBuffer of buffers) {
					for (const pass of commandBufferPasses.get(commandBuffer) ?? []) {
						passes.push(Object.freeze({ ...pass, submissionId }));
					}
				}
				originalSubmit.call(device.queue, buffers);
			};
			const originalCreateCommandEncoder = device.createCommandEncoder;
			device.createCommandEncoder = descriptor => {
				const encoder = originalCreateCommandEncoder.call(device, descriptor);
				const encodedPasses: Omit<GpuRenderPassTrace, 'submissionId'>[] = [];
				const originalBeginRenderPass = encoder.beginRenderPass;
				encoder.beginRenderPass = descriptor => {
					const label = descriptor.label ?? '';
					if (label === 'Stanza rectangle pass' || label === 'Stanza ViewLinesGpu pass') {
						const attachment = [...descriptor.colorAttachments][0];
						if (attachment) {
							let viewId = viewIds.get(attachment.view);
							if (viewId === undefined) {
								viewId = nextViewId;
								nextViewId += 1;
								viewIds.set(attachment.view, viewId);
							}
							encodedPasses.push(Object.freeze({ label, loadOp: attachment.loadOp, viewId }));
						}
					}
					return originalBeginRenderPass.call(encoder, descriptor);
				};
				const originalFinish = encoder.finish;
				encoder.finish = descriptor => {
					const commandBuffer = originalFinish.call(encoder, descriptor);
					commandBufferPasses.set(commandBuffer, encodedPasses);
					return commandBuffer;
				};
				return encoder;
			};
			return device;
		};
		return adapter;
	};
	return {
		reset: () => {
			passes = [];
			viewIds = new WeakMap();
			nextViewId = 1;
			nextSubmissionId = 1;
		},
		read: () => Object.freeze([...passes]),
		dispose: () => {
			gpu.requestAdapter = originalRequestAdapter;
		},
	};
}

function requiredElement(id: string): HTMLElement {
	const element = document.getElementById(id);
	if (!(element instanceof HTMLElement)) throw new Error(`Missing GPU integration element '#${id}'`);
	return element;
}
