[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[block]]
struct Camera {
    origin: vec4<f32>;
    view_direction: vec4<f32>;
    up: vec4<f32>;
    view_matrix: mat4x4<f32>;
};
[[group(0), binding(2)]]
var<uniform> camera: Camera;

[[block]]
struct Settings {
    field_weight: f32;
};
[[group(0), binding(3)]]
var<uniform> settings: Settings;

struct Vertex {
    x: f32;
    y: f32;
    z: f32;
};
[[block]]
struct Vertices {
    data: [[stride(12)]] array<Vertex>;
};
[[group(1), binding(0)]]
var<storage> vertices: [[access(read)]] Vertices;

struct Face {
    a: u32;
    b: u32;
    c: u32;
};
[[block]]
struct Faces {
    data: [[stride(12)]] array<Face>;
};
[[group(1), binding(1)]]
var<storage> faces: [[access(read)]] Faces;

struct RaySample {
    position: vec4<f32>;
    direction: vec4<f32>;
};
[[block]]
struct RaySamples {
    data: [[stride(32)]] array<RaySample, 1000>;
};
[[group(2), binding(0)]]
var<storage> ray_samples: [[access(write)]] RaySamples;

let backface_culling: bool = false;

let light_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
let ambient_strength: f32 = 0.01;
let shininess: f32 = 64.0;
let object_color: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);

let linear_mode: bool = false;
let use_lighting: bool = true;
let eps: f32 = 0.0000001;

let PI: f32 = 3.141592653589793;

fn rotateX(v: vec3<f32>, phi: f32) -> vec3<f32> {
    return mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, cos(phi), sin(phi)),
        vec3<f32>(0.0, -sin(phi), cos(phi)),
    ) * v;
}

fn rotateY(v: vec3<f32>, phi: f32) -> vec3<f32> {
    return mat3x3<f32>(
        vec3<f32>(cos(phi), 0.0, -sin(phi)),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(sin(phi), 0.0, cos(phi)),
    ) * v;
}

fn rotateZ(v: vec3<f32>, phi: f32) -> vec3<f32> {
    return mat3x3<f32>(
        vec3<f32>(cos(phi), sin(phi), 0.0),
        vec3<f32>(-sin(phi), cos(phi), 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
    ) * v;
}

fn translate(v: vec3<f32>, dx: f32, dy: f32, dz: f32) -> vec3<f32> {
    return vec3<f32>(
        v.x + dx,
        v.y + dy,
        v.z + dz,
    );
}

fn field_function(p: vec3<f32>, v: vec3<f32>) -> vec3<f32> { return v; }

fn vector_fn(p: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    return (1.0 - settings.field_weight) * v + settings.field_weight * field_function(p, v);
}

fn hit_triangle(v: array<vec3<f32>, 3>, origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    // Möller-Trumbore intersection algorithm
    let edge1 = v[1] - v[0];
    let edge2 = v[2] - v[0];
    let h = cross(direction, edge2);
    let a = dot(edge1, h);
    if ((backface_culling || a > -eps) && a < eps) {
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

fn ray_color(origin: vec3<f32>, direction: vec3<f32>, max_dist: f32) -> vec4<f32> {
    var t: f32 = -1.0;
    var t_new: f32;
    var d1: vec3<f32>;
    var d2: vec3<f32>;
    for (var i: u32 = 0u; i < arrayLength(&faces.data); i = i + 1u) {
        let face = faces.data[i];
        let a = vertices.data[face.a];
        let b = vertices.data[face.b];
        let c = vertices.data[face.c];
        let triangle = array<vec3<f32>, 3>(
            (camera.view_matrix * vec4<f32>(a.x, a.y, a.z, 1.0)).xyz,
            (camera.view_matrix * vec4<f32>(b.x, b.y, b.z, 1.0)).xyz,
            (camera.view_matrix * vec4<f32>(c.x, c.y, c.z, 1.0)).xyz,
        );
        t_new = hit_triangle(triangle, origin, direction);
        if (t_new > 0.0 && t_new < max_dist && (t < 0.0 || t_new < t)) {
            t = t_new;
            d1 = triangle[1] - triangle[0];
            d2 = triangle[2] - triangle[0];
        }
    }
    if (t > 0.0 && t < max_dist) {
        let normal = normalize(cross(d1, d2));
        if (!use_lighting) {
            let color = abs(normal);
            return vec4<f32>(color, 1.0);
        }
        let ambient = ambient_strength * light_color;
        // The camera is the light source here, which allows for
        // some simplifications
        let intensity = max(dot(normal, -direction), 0.0);
        let diff = intensity;
        let diffuse = diff * light_color;
        let spec = pow(intensity, shininess);
        let specular = spec * light_color;
        let result = (ambient + diffuse + specular) * abs(normal);
        return vec4<f32>(result, t);
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

fn nonlinear_ray_color(start_point: vec3<f32>, start_dir: vec3<f32>) -> vec4<f32> {
    let h = 0.25;
    let steps = 100;
    var cur_point: vec3<f32> = start_point;
    var cur_dir: vec3<f32> = start_dir;
    var color: vec4<f32>;

    for (var i: i32 = 0; i < steps; i = i + 1) {
        if (i % 2 == 0) {
            // draw_reference_point(cur_point, vec4<f32>(1.0, 1.0, 1.0, 1.0));
        }

        // Runge-Kutta method
        let k1 = vector_fn(cur_point, cur_dir);
        let k2 = vector_fn(cur_point + 0.5 * h * k1, 0.5 * h * k1);
        let k3 = vector_fn(cur_point + 0.5 * h * k2, 0.5 * h * k2);
        let k4 = vector_fn(cur_point + h * k3, h * k3);
        cur_dir = h / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);

        let unit_dir = normalize(cur_dir);
        color = ray_color(cur_point, unit_dir, length(cur_dir));
        if (color.a > 0.0) {
            cur_point = cur_point + color.a * unit_dir;
            color.a = 1.0;
            // draw_reference_point(cur_point, color);
            return color;
        }

        cur_point = cur_point + cur_dir;
        cur_dir = normalize(k4);
    }

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y));
    if (coords.x >= size.x || coords.y < 0) {
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

    let x_step = size.x / 10;
    let y_step = size.y / 10;
    if (coords.x % x_step == 0 && coords.y % y_step == 0) {
        var sample: RaySample;
        sample.position = vec4<f32>(origin.xyz, 0.0);
        sample.direction = vec4<f32>(dir, 0.0);
        for (var i: i32 = 0; i < 10; i = i + 1) {
            sample.position = sample.position + 0.1 * sample.direction;
            let index = coords.x / x_step * 100 + coords.y / y_step * 10 + i;
            ray_samples.data[index] = sample;
        }
    }

    if (linear_mode) {
        let color = ray_color(origin, dir, 100.0);
        textureStore(target, coords, color);
    } else {
        let color = nonlinear_ray_color(origin, dir);
        textureStore(target, coords, color);
    }
}
