// --- begin: translated from linalg.h

fn mat2det(a: mat2x2<f32>) -> f32 {
    return a[0].x * a[1].y - a[0].y * a[1].x;
}

fn mat2invariants(m: mat2x2<f32>) -> vec2<f32> {
    return vec2<f32>(
        mat2det(m),
        -(m[0].x + m[1].y)
    );
}

fn vec2squareroots(a: vec2<f32>) -> vec2<f32> {
  // Solves equation
  //    1 * x^2 + a[1]*x + a[0] = 0
  //
  // On output, 
  //    r[0], r[1] or
  //    r[0] +- i*r[1] are the roots 
  let discrim = a[1] * a[1] - 4.0 * a[0];
  if (discrim >= 0.0) {
      let root = sqrt(discrim);
      return vec2<f32>(
        (-a[1] - root) / 2.0,
        (-a[1] + root) / 2.0
      );
  }
  let root = sqrt(-discrim);
  return vec2<f32>(
    -a[1] / 2.0,
    root / 2.0,
  );
}

fn mat2eigenvalues(m: mat2x2<f32>) -> vec2<f32> {
    let pqr = mat2invariants(m);
    return vec2squareroots(pqr);
}

// --- end: translated from linalg.h


[[group(0), binding(0)]]
var ray_casting: texture_2d<f32>;
[[group(0), binding(1)]]
var mapping: texture_2d<f32>;
[[group(0), binding(2)]]
var target: texture_storage_2d<rgba8unorm, write>;

[[block]]
struct Settings {
    field_weight: f32;
    mouse_pos_x: f32;
    mouse_pos_y: f32;
    show_lyapunov_exponent: i32;
    central_difference_delta: i32;
    lyapunov_scaling: f32;
};
[[group(0), binding(3)]]
var<uniform> settings: Settings;

fn lyapunov(coords: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    let color = textureLoad(ray_casting, coords, 0);
    if (settings.show_lyapunov_exponent == 0) {
        return color;
    }
    let delta = settings.central_difference_delta;
    if (coords.x < delta || coords.y < delta || coords.x >= size.x - delta || coords.y >= size.y - delta) {
        return color;
    }
    let x_next = textureLoad(mapping, vec2<i32>(coords.x + delta, coords.y), 0).xyz;
    let x_prev = textureLoad(mapping, vec2<i32>(coords.x - delta, coords.y), 0).xyz;
    let y_next = textureLoad(mapping, vec2<i32>(coords.x, coords.y + delta), 0).xyz;
    let y_prev = textureLoad(mapping, vec2<i32>(coords.x, coords.y - delta), 0).xyz;
    let gradient = mat2x3<f32>(
        1.0/f32(delta * 2) * (x_next - x_prev),
        1.0/f32(delta * 2) * (y_next - y_prev)
    );
    let m = transpose(gradient) * gradient;
    let eigen = mat2eigenvalues(m);
    let exponent = sqrt(max(eigen[0], eigen[1]));
    let c = exp(settings.lyapunov_scaling * exponent - 5.0);
    return c * vec4<f32>(1.0, 1.0, 1.0, 1.0) + (1.0 - c) * color;
}

[[stage(compute), workgroup_size(8, 8)]]
fn lyapunov_desktop([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y) - 1);
    if (coords.x >= size.x || coords.y < 0) {
        return;
    }
    let color = lyapunov(coords, size);
    textureStore(target, coords, color);
}

fn srgb_from_linear(linear_rgb: vec3<f32>) -> vec3<f32> {
    // Based on https://gamedev.stackexchange.com/a/148088
    let cutoff = linear_rgb < vec3<f32>(0.0031308);
    let lower = linear_rgb / vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(linear_rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

[[stage(compute), workgroup_size(8, 8)]]
fn lyapunov_web([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y) - 1);
    if (coords.x >= size.x || coords.y < 0) {
        return;
    }
    let color = lyapunov(coords, size);
    textureStore(target, coords, vec4<f32>(srgb_from_linear(color.rgb), color.a));
}
