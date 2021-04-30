[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

fn hit_sphere(center: vec3<f32>, radius: f32, origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    let oc = origin - center;
    let a = dot(direction, direction);
    let half_b = dot(oc, direction);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = half_b * half_b - a * c;
    if (discriminant < 0.0) {
        return -1.0;
    }
    return (-half_b - sqrt(discriminant)) / a;
}

fn ray_color(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    var t: f32 = hit_sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5, origin, direction);
    if (t > 0.0) {
        let N = normalize(origin + t * direction - vec3<f32>(0.0, 0.0, -1.0));
        return 0.5 * vec4<f32>(N.x + 1.0, N.y + 1.0, N.z + 1.0, 1.0);
    }
    let ndir = normalize(direction);
    t = 0.5 * (ndir.y + 1.0);
    return (1.0 - t) * vec4<f32>(1.0, 1.0, 1.0, 1.0) + t * vec4<f32>(0.5, 0.7, 1.0, 1.0);
}

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    let size = textureDimensions(target);
    if (coords.x >= size.x || coords.y >= size.y) {
        return;
    }

    let width = f32(size.x);
    let height = f32(size.y);

    let aspect_ratio = width / height;
    let viewport_height = 2.0;
    let viewport_width = aspect_ratio * viewport_height;
    let focal_length = 1.0;

    let origin = vec3<f32>(0.0, 0.0, 0.0);
    let horizontal = vec3<f32>(viewport_width, 0.0, 0.0);
    let vertical = vec3<f32>(0.0, viewport_height, 0.0);
    let lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0 - vec3<f32>(0.0, 0.0, focal_length);

    let u = f32(gid.x) / (width - 1.0);
    let v = f32(gid.y) / (height - 1.0);
    let dir = lower_left_corner + u * horizontal + v * vertical - origin;
    let color = ray_color(origin, dir);

    textureStore(target, coords, color);
}
