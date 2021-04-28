[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    let color = vec4<f32>(f32(gid.x) / 800.0, f32(gid.y) / 600.0, 0.5, 1.0);
    textureStore(target, coords, color);
}
