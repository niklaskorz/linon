[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[block]]
struct Camera {
    origin: vec4<f32>;
    view_direction: vec4<f32>;
    up: vec4<f32>;
};
[[group(0), binding(1)]]
var<uniform> camera: Camera;

[[block]]
struct NumFaces {
    length: u32; // Needed until arrayLength lands in wgpu-rs
};
[[group(0), binding(2)]]
var<uniform> num_faces: NumFaces;

[[block]]
struct Vertices {
    data: [[stride(12)]] array<f32>;
};
[[group(1), binding(0)]]
var<storage> vertices: [[access(read)]] Vertices;

[[block]]
struct Faces {
    data: [[stride(12)]] array<array<u32, 3>>;
};
[[group(1), binding(1)]]
var<storage> faces: [[access(read)]] Faces;

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

let eps: f32 = 0.0000001;

fn hit_triangle(v: array<vec3<f32>, 3>, origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    let edge1 = v[1] - v[0];
    let edge2 = v[2] - v[0];
    let h = cross(direction, edge2);
    let a = dot(edge1, h);
    if (a > -eps && a < eps) {
        return -1.0;
    }
    let f = 1.0 / a;
    let s = origin - v[0];
    let u = f * dot(s, h);
    if (u < 0.0 || u > 1.0) {
        return -1.0;
    }
    let q = cross(s, edge1);
    let v = f * dot(direction, q);
    if (v < 0.0 || u + v > 1.0) {
        return -1.0;
    }
    let t = f * dot(edge2, q);
    if (t > eps) {
        return t;
    }
    return -1.0;
}

fn ray_color(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    var t: f32 = -1.0;
    var t_new: f32;
    var d1: vec3<f32>;
    var d2: vec3<f32>;
    for (var i: u32 = 0u; i < num_faces.length; i = i + 1u) {
        let indices = faces.data[i];
        let x = indices[0];
        let y = indices[1];
        let z = indices[2];
        let triangle = array<vec3<f32>, 3>(
            vec3<f32>(vertices.data[3u * x], vertices.data[3u * x + 1u], vertices.data[3u * x + 2u]),
            vec3<f32>(vertices.data[3u * y], vertices.data[3u * y + 1u], vertices.data[3u * y + 2u]),
            vec3<f32>(vertices.data[3u * z], vertices.data[3u * z + 1u], vertices.data[3u * z + 2u]),
        );
        t_new = hit_triangle(triangle, origin, direction);
        if (t_new > 0.0 && (t < 0.0 || t_new < t)) {
            t = t_new;
            d1 = triangle[1] - triangle[0];
            d2 = triangle[2] - triangle[0];
        }
    }
    if (t > 0.0) {
        let normal = normalize(cross(d1, d2));
        let color = abs(normal);
        return vec4<f32>(color, 1.0);
    }
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
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

    // Camera properties
    let origin = camera.origin.xyz;
    let view_direction = camera.view_direction.xyz;
    let up = camera.up.xyz;
    let focal_length = 0.035;
    let viewport_height = 0.025;
    let viewport_width = aspect_ratio * viewport_height;

    let w = -view_direction;
    let horizontal = cross(up, w);
    let vertical = cross(w, horizontal);

    let u = f32(gid.x) / (width - 1.0) * viewport_width - 0.5 * viewport_width;
    let v = f32(gid.y) / (height - 1.0) *  viewport_height - 0.5 * viewport_height;
    let s = u * normalize(horizontal) + v * normalize(vertical) + focal_length * view_direction;
    let dir = normalize(s);
    let color = ray_color(origin, dir);

    textureStore(target, coords, color);
}
