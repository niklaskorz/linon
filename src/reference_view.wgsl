[[block]]
struct Uniforms {
    view_projection: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;

struct VertexInput {
    [[location(0)]]
    position: vec3<f32>;
};
struct VertexOutput {
    [[builtin(position)]]
    position: vec4<f32>;  
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.view_projection * vec4<f32>(input.position, 1.0);
    return output;
}

[[stage(fragment)]]
fn main(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
