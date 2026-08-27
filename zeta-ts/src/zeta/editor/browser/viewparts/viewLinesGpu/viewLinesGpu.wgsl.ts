export const VIEW_LINES_GPU_SHADER = /* wgsl */ `
struct Dimensions {
  viewport: vec2f,
  atlas: vec2f,
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) page: f32,
}

@group(0) @binding(0) var atlasTexture: texture_2d_array<f32>;
@group(0) @binding(1) var atlasSampler: sampler;
@group(0) @binding(2) var<uniform> dimensions: Dimensions;

@vertex
fn vertexMain(
  @location(0) position: vec2f,
  @location(1) atlasPosition: vec2f,
  @location(2) page: f32,
) -> VertexOutput {
  var output: VertexOutput;
  let normalized = position / dimensions.viewport;
  output.position = vec4f(normalized.x * 2.0 - 1.0, 1.0 - normalized.y * 2.0, 0.0, 1.0);
  output.uv = atlasPosition / dimensions.atlas;
  output.page = page;
  return output;
}

@fragment
fn fragmentMain(input: VertexOutput) -> @location(0) vec4f {
  return textureSample(atlasTexture, atlasSampler, input.uv, i32(input.page));
}
`;
