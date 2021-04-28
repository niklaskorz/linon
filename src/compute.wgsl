[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    var color: vec4<f32> = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    if (coords.x < 50 && coords.y < 50) {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (coords.x > (800 - 50) && coords.y < 50) {
        color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (coords.x > (800 - 50) && coords.y > (600 - 50)) {
        color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (coords.x < 50 && coords.y > (600 - 50)) {
        color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    textureStore(target, coords, color);
}
