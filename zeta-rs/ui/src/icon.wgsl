@group(0) @binding(0)
var icon_atlas: texture_2d<f32>;

@group(0) @binding(1)
var icon_sampler: sampler;

struct IconInstance {
    @location(0) bounds: vec4<f32>,
    @location(1) uv_bounds: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) clip_bounds: vec4<f32>,
    @location(4) viewport: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) screen_position: vec2<f32>,
    @location(1) atlas_position: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) clip_bounds: vec4<f32>,
};

const QUAD_VERTICES = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(instance: IconInstance, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let unit_position = QUAD_VERTICES[vertex_index];
    let screen_position = instance.bounds.xy + unit_position * instance.bounds.zw;
    let ndc = screen_position / instance.viewport.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.screen_position = screen_position;
    output.atlas_position = instance.uv_bounds.xy + unit_position * instance.uv_bounds.zw;
    output.color = instance.color;
    output.clip_bounds = instance.clip_bounds;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let clip_max = input.clip_bounds.xy + input.clip_bounds.zw;
    if input.screen_position.x < input.clip_bounds.x
        || input.screen_position.y < input.clip_bounds.y
        || input.screen_position.x >= clip_max.x
        || input.screen_position.y >= clip_max.y {
        discard;
    }
    let mask = textureSample(icon_atlas, icon_sampler, input.atlas_position).r;
    let alpha = mask * input.color.a;
    if alpha <= 0.0 {
        discard;
    }
    return vec4<f32>(input.color.rgb, alpha);
}
