[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

let width: f32 = 1280.0;
let height: f32 = 720.0;

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    let color = vec4<f32>(f32(gid.x) / width, f32(gid.y) / height, 0.5, 1.0);
    textureStore(target, coords, color);
}
