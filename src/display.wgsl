let positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
);

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] in_vertex_index: u32) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
}

[[group(0), binding(0)]]
var texture: texture_2d<f32>;

[[stage(fragment)]]
fn fs_main([[builtin(position)]] position: vec4<f32>) -> [[location(0)]] vec4<f32> {
    let size = textureDimensions(texture);
    let x = i32(position.x);
    let y = size.y - 1 - i32(position.y);
    return textureLoad(texture, vec2<i32>(x, y), 0);
}
