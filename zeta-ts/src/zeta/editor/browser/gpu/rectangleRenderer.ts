import { Disposable, MutableDisposable, type IReference } from '../../../base/common/lifecycle.js';
import { GPULifecycle } from './gpuDisposable.js';
import { quadVertices } from './gpuUtils.js';
import { createObjectCollectionBuffer, type IObjectCollectionBuffer, type IObjectCollectionBufferEntry } from './objectCollectionBuffer.js';
import { RectangleRendererBindingId, rectangleRendererWgsl } from './rectangleRenderer.wgsl.js';

export type RectangleRendererEntrySpec = [
	{ name: 'x' },
	{ name: 'y' },
	{ name: 'width' },
	{ name: 'height' },
	{ name: 'red' },
	{ name: 'green' },
	{ name: 'blue' },
	{ name: 'alpha' },
];

const rectangleProperties: RectangleRendererEntrySpec = [
	{ name: 'x' }, { name: 'y' }, { name: 'width' }, { name: 'height' },
	{ name: 'red' }, { name: 'green' }, { name: 'blue' }, { name: 'alpha' },
];

export class RectangleRenderer extends Disposable {
	private readonly shapes: IObjectCollectionBuffer<RectangleRendererEntrySpec> = this._register(createObjectCollectionBuffer(rectangleProperties, 32));
	private readonly shapeBuffer = this._register(new MutableDisposable<IReference<GPUBuffer>>());
	private readonly vertexBuffer: GPUBuffer;
	private readonly layoutBuffer: GPUBuffer;
	private readonly scrollBuffer: GPUBuffer;
	private readonly pipeline: GPURenderPipeline;
	private bindGroup: GPUBindGroup;

	constructor(private readonly device: GPUDevice, presentationFormat: GPUTextureFormat) {
		super();
		this.vertexBuffer = this._register(GPULifecycle.createBuffer(device, {
			label: 'Stanza rectangle vertices',
			size: quadVertices.byteLength,
			usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
		}, quadVertices as Float32Array<ArrayBuffer>)).object;
		this.layoutBuffer = this._register(GPULifecycle.createBuffer(device, {
			label: 'Stanza rectangle layout',
			size: 6 * Float32Array.BYTES_PER_ELEMENT,
			usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
		})).object;
		this.scrollBuffer = this._register(GPULifecycle.createBuffer(device, {
			label: 'Stanza rectangle scroll offset',
			size: 2 * Float32Array.BYTES_PER_ELEMENT,
			usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
		})).object;
		const module = device.createShaderModule({ label: 'Stanza rectangle shader', code: rectangleRendererWgsl });
		this.pipeline = device.createRenderPipeline({
			label: 'Stanza rectangle pipeline',
			layout: 'auto',
			vertex: { module, entryPoint: 'vs', buffers: [{ arrayStride: 2 * Float32Array.BYTES_PER_ELEMENT, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x2' }] }] },
			fragment: { module, entryPoint: 'fs', targets: [{ format: presentationFormat, blend: { color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' }, alpha: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' } } }] },
		});
		this.createShapeBuffer();
		this.bindGroup = this.createBindGroup();
		this._register(this.shapes.onDidChangeBuffer(() => {
			this.createShapeBuffer();
			this.bindGroup = this.createBindGroup();
		}));
	}

	public register(x: number, y: number, width: number, height: number, red: number, green: number, blue: number, alpha: number): IObjectCollectionBufferEntry<RectangleRendererEntrySpec> {
		return this.shapes.createEntry({ x, y, width, height, red, green, blue, alpha });
	}

	public encode(encoder: GPUCommandEncoder, view: GPUTextureView, width: number, height: number, scrollLeft: number, scrollTop: number): void {
		this.updateShapeBuffer();
		this.device.queue.writeBuffer(this.layoutBuffer, 0, new Float32Array([width, height, 0, 0, width, height]));
		this.device.queue.writeBuffer(this.scrollBuffer, 0, new Float32Array([scrollLeft, scrollTop]));
		const pass = encoder.beginRenderPass({
			label: 'Stanza rectangle pass',
			colorAttachments: [{ view, clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' }],
		});
		if (this.shapes.entryCount > 0) {
			pass.setPipeline(this.pipeline);
			pass.setVertexBuffer(0, this.vertexBuffer);
			pass.setBindGroup(0, this.bindGroup);
			pass.draw(quadVertices.length / 2, this.shapes.entryCount);
		}
		pass.end();
	}

	private createShapeBuffer(): void {
		this.shapeBuffer.value = GPULifecycle.createBuffer(this.device, {
			label: 'Stanza rectangle shapes',
			size: this.shapes.buffer.byteLength,
			usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
		});
		if (this.shapes.viewUsedSize > 0) this.device.queue.writeBuffer(this.shapeBuffer.value.object, 0, this.shapes.view as Float32Array<ArrayBuffer>, 0, this.shapes.viewUsedSize);
		this.shapes.dirtyTracker.clear();
	}

	private createBindGroup(): GPUBindGroup {
		return this.device.createBindGroup({
			label: 'Stanza rectangle bindings',
			layout: this.pipeline.getBindGroupLayout(0),
			entries: [
				{ binding: RectangleRendererBindingId.Shapes, resource: { buffer: this.shapeBuffer.value!.object } },
				{ binding: RectangleRendererBindingId.LayoutInfoUniform, resource: { buffer: this.layoutBuffer } },
				{ binding: RectangleRendererBindingId.ScrollOffset, resource: { buffer: this.scrollBuffer } },
			],
		});
	}

	private updateShapeBuffer(): void {
		const dirty = this.shapes.dirtyTracker;
		if (!dirty.isDirty || dirty.dataOffset === undefined || dirty.dirtySize === undefined) return;
		this.device.queue.writeBuffer(this.shapeBuffer.value!.object, dirty.dataOffset * Float32Array.BYTES_PER_ELEMENT, this.shapes.view as Float32Array<ArrayBuffer>, dirty.dataOffset, dirty.dirtySize);
		dirty.clear();
	}
}
