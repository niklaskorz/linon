[[group(0), binding(0)]]
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

let white: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
let grey: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);
let green: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);
let red: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
let blue: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);
let yellow: vec3<f32> = vec3<f32>(1.0, 1.0, 0.0);
let brown: vec3<f32> = vec3<f32>(0.59, 0.29, 0.0);

let num_quads: i32 = 18;
let num_vertices: i32 = 72; // num_quads * 4
let vertices: array<vec3<f32>, num_vertices> = array<vec3<f32>, num_vertices>(
    // Floor
    vec3<f32>(552.8, 0.0,   0.0),
    vec3<f32>(0.0,   0.0,   0.0),
    vec3<f32>(0.0,   0.0, 559.2),
    vec3<f32>(549.6, 0.0, 559.2),

    vec3<f32>(130.0, 0.0,  65.0),
    vec3<f32>( 82.0, 0.0, 225.0),
    vec3<f32>(240.0, 0.0, 272.0),
    vec3<f32>(290.0, 0.0, 114.0),

    vec3<f32>(423.0, 0.0, 247.0),
    vec3<f32>(265.0, 0.0, 296.0),
    vec3<f32>(314.0, 0.0, 456.0),
    vec3<f32>(472.0, 0.0, 406.0),

    // Ceiling
    vec3<f32>(556.0, 548.8,   0.0),
    vec3<f32>(556.0, 548.8, 559.2),
    vec3<f32>(  0.0, 548.8, 559.2),
    vec3<f32>(  0.0, 548.8,   0.0),

    vec3<f32>(343.0, 548.8, 227.0),
    vec3<f32>(343.0, 548.8, 332.0),
    vec3<f32>(213.0, 548.8, 332.0),
    vec3<f32>(213.0, 548.8, 227.0),

    // Back wall
    vec3<f32>(549.6,   0.0, 559.2),
    vec3<f32>(  0.0,   0.0, 559.2),
    vec3<f32>(  0.0, 548.8, 559.2),
    vec3<f32>(556.0, 548.8, 559.2),

    // Right wall
    vec3<f32>(0.0,   0.0, 559.2),
    vec3<f32>(0.0,   0.0,   0.0),
    vec3<f32>(0.0, 548.8,   0.0),
    vec3<f32>(0.0, 548.8, 559.2),

    // Left wall
    vec3<f32>(552.8,   0.0,   0.0),
    vec3<f32>(549.6,   0.0, 559.2),
    vec3<f32>(556.0, 548.8, 559.2),
    vec3<f32>(556.0, 548.8,   0.0),

    // Short block
    vec3<f32>(130.0, 165.0,  65.0),
    vec3<f32>( 82.0, 165.0, 225.0),
    vec3<f32>(240.0, 165.0, 272.0),
    vec3<f32>(290.0, 165.0, 114.0),

    vec3<f32>(290.0,   0.0, 114.0),
    vec3<f32>(290.0, 165.0, 114.0),
    vec3<f32>(240.0, 165.0, 272.0),
    vec3<f32>(240.0,   0.0, 272.0),

    vec3<f32>(130.0,   0.0,  65.0),
    vec3<f32>(130.0, 165.0,  65.0),
    vec3<f32>(290.0, 165.0, 114.0),
    vec3<f32>(290.0,   0.0, 114.0),

    vec3<f32>( 82.0,   0.0, 225.0),
    vec3<f32>( 82.0, 165.0, 225.0),
    vec3<f32>(130.0, 165.0,  65.0),
    vec3<f32>(130.0,   0.0,  65.0),

    vec3<f32>(240.0,   0.0, 272.0),
    vec3<f32>(240.0, 165.0, 272.0),
    vec3<f32>( 82.0, 165.0, 225.0),
    vec3<f32>( 82.0,   0.0, 225.0),

    // Tall block
    vec3<f32>(423.0, 330.0, 247.0),
    vec3<f32>(265.0, 330.0, 296.0),
    vec3<f32>(314.0, 330.0, 456.0),
    vec3<f32>(472.0, 330.0, 406.0),

    vec3<f32>(423.0,   0.0, 247.0),
    vec3<f32>(423.0, 330.0, 247.0),
    vec3<f32>(472.0, 330.0, 406.0),
    vec3<f32>(472.0,   0.0, 406.0),

    vec3<f32>(472.0,   0.0, 406.0),
    vec3<f32>(472.0, 330.0, 406.0),
    vec3<f32>(314.0, 330.0, 456.0),
    vec3<f32>(314.0,   0.0, 456.0),

    vec3<f32>(314.0,   0.0, 456.0),
    vec3<f32>(314.0, 330.0, 456.0),
    vec3<f32>(265.0, 330.0, 296.0),
    vec3<f32>(265.0,   0.0, 296.0),

    vec3<f32>(265.0,   0.0, 296.0),
    vec3<f32>(265.0, 330.0, 296.0),
    vec3<f32>(423.0, 330.0, 247.0),
    vec3<f32>(423.0,   0.0, 247.0),
);
let colors: array<vec3<f32>, num_quads> = array<vec3<f32>, num_quads>(
    // Floor
    grey,
    grey,
    grey,
    // Ceiling
    white,
    white,
    // Back wall
    blue,
    // Right wall
    green,
    // Left wall
    red,
    // Short block
    yellow,
    yellow,
    yellow,
    yellow,
    yellow,
    // Tall block
    brown,
    brown,
    brown,
    brown,
    brown,
);

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
    var ic: i32;
    for (var i: i32 = 0; i < num_quads; i = i + 1) {
        let t0 = array<vec3<f32>, 3>(vertices[(i*4)], vertices[(i*4)+1], vertices[(i*4)+2]);
        let t1 = array<vec3<f32>, 3>(vertices[(i*4)+2], vertices[(i*4)+3], vertices[(i*4)]);
        t_new = hit_triangle(t0, origin, direction);
        if (t_new < 0.0) {
            t_new = hit_triangle(t1, origin, direction);
        }
        if (t_new > 0.0 && (t < 0.0 || t_new < t)) {
            t = t_new;
            ic = i; 
        }
    }
    if (t > 0.0) {
        let normal = normalize(cross(vertices[(ic*4)+1] - vertices[(ic*4)], vertices[(ic*4)+2] - vertices[(ic*4)]));
        let color = abs(normal);
        return vec4<f32>(color, 1.0);
        // return vec4<f32>(colors[ic], 1.0);
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
    let origin = vec3<f32>(278.0, 273.0, -800.0);
    let view_direction = vec3<f32>(0.0, 0.0, 1.0);
    let focal_length = 0.035;
    let viewport_height = 0.025;
    let viewport_width = aspect_ratio * viewport_height;

    let horizontal = vec3<f32>(-1.0, 0.0, 0.0);
    let vertical = vec3<f32>(0.0, 1.0, 0.0);

    let u = f32(gid.x) / (width - 1.0) * viewport_width - 0.5 * viewport_width;
    let v = f32(gid.y) / (height - 1.0) *  viewport_height - 0.5 * viewport_height;
    let s = origin + u * horizontal + v * vertical + focal_length * view_direction;
    let dir = normalize(s - origin);
    let color = ray_color(origin, dir);

    textureStore(target, coords, color);
}
