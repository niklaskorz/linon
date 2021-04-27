[[group(0), binding(0)]]
var result: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    textureStore(result, coords, vec4<f32>(0.0, 0.0, 1.0, 1.0));
}
