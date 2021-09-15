// --- begin: translated from linalg.h (originally in C / GLSL)
// Linear Algebra Library by Ronald Peikert (CGL ETH Zurich)

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

fn vec2mag(a: vec2<f32>) -> f32 {
    return sqrt(dot(a, a));
}

fn vec2nrm(a: vec2<f32>) -> vec2<f32> {
    var l: f32 = vec2mag(a);
    if (l == 0.0) {
        l = 1.0;
    }
    return vec2<f32>(a.x / l, a.y / l);
}

fn mat2realEigenvector(m: mat2x2<f32>, lambda: f32) -> vec2<f32> {
    var reduced: mat2x2<f32> = m;
    reduced[0].x = reduced[0].x - lambda;
    reduced[1].y = reduced[1].y - lambda;
    var ev: vec2<f32>;
    if (vec2mag(reduced[1]) > vec2mag(reduced[0])) {
        ev.x = reduced[1].y;
        ev.y = -reduced[1].x;
    } else {
        ev.x = reduced[0].y;
        ev.y = -reduced[0].x;
    }
    if (vec2mag(ev) == 0.0) {
        return ev;
    }
    return vec2nrm(ev);
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
    overlay_mode: i32;
    central_difference_delta: i32;
    lyapunov_scaling: f32;
};
[[group(0), binding(3)]]
var<uniform> settings: Settings;

fn lyapunov_exponent(coords: vec2<i32>) -> f32 {
    let x_next = textureLoad(mapping, coords + vec2<i32>(settings.central_difference_delta, 0), 0).xyz;
    let x_prev = textureLoad(mapping, coords - vec2<i32>(settings.central_difference_delta, 0), 0).xyz;
    let y_next = textureLoad(mapping, coords + vec2<i32>(0, settings.central_difference_delta), 0).xyz;
    let y_prev = textureLoad(mapping, coords - vec2<i32>(0, settings.central_difference_delta), 0).xyz;
    let gradient = mat2x3<f32>(
        0.5 * (x_next - x_prev),
        0.5 * (y_next - y_prev)
    );
    let m = transpose(gradient) * gradient;
    let eigen = mat2eigenvalues(m);
    let exponent = sqrt(max(eigen[0], eigen[1]));
    return exponent;
}

fn f_x(coords: vec2<i32>) -> f32 {
    let x_next = lyapunov_exponent(coords + vec2<i32>(settings.central_difference_delta, 0));
    let x_prev = lyapunov_exponent(coords - vec2<i32>(settings.central_difference_delta, 0));
    return 0.5 * (x_next - x_prev);
}

fn f_y(coords: vec2<i32>) -> f32 {
    let y_next = lyapunov_exponent(coords + vec2<i32>(0, settings.central_difference_delta));
    let y_prev = lyapunov_exponent(coords - vec2<i32>(0, settings.central_difference_delta));
    return 0.5 * (y_next - y_prev);
}

fn f_x_x(coords: vec2<i32>) -> f32 {
    let x_next = f_x(coords + vec2<i32>(settings.central_difference_delta, 0));
    let x_prev = f_x(coords - vec2<i32>(settings.central_difference_delta, 0));
    return 0.5 * (x_next - x_prev);
}

fn f_x_y(coords: vec2<i32>) -> f32 {
    let y_next = f_x(coords + vec2<i32>(0, settings.central_difference_delta));
    let y_prev = f_x(coords - vec2<i32>(0, settings.central_difference_delta));
    return 0.5 * (y_next - y_prev);
}

fn f_y_x(coords: vec2<i32>) -> f32 {
    let x_next = f_y(coords + vec2<i32>(settings.central_difference_delta, 0));
    let x_prev = f_y(coords - vec2<i32>(settings.central_difference_delta, 0));
    return 0.5 * (x_next - x_prev);
}

fn f_y_y(coords: vec2<i32>) -> f32 {
    let y_next = f_y(coords + vec2<i32>(0, settings.central_difference_delta));
    let y_prev = f_y(coords - vec2<i32>(0, settings.central_difference_delta));
    return 0.5 * (y_next - y_prev);
}

fn hessian(coords: vec2<i32>) -> mat2x2<f32> {
    return mat2x2<f32>(
        vec2<f32>(f_x_x(coords), f_x_y(coords)),
        vec2<f32>(f_y_x(coords), f_y_y(coords))
    );
}

fn ridge_test(coords: vec2<i32>, size: vec2<i32>) -> bool {
    let hes = hessian(coords);
    let eigenvalues = mat2eigenvalues(hes);
    let minor = max(eigenvalues.x, eigenvalues.y);
    if (minor >= 0.0) {
        return false;
    }
    let eigenvector = mat2realEigenvector(hes, minor);
    let gradient = vec2<f32>(
        f_x(coords), f_y(coords)
    );
    if (abs(dot(eigenvector, gradient)) < 0.0001) {
        return false;
    }
    return true;
}

fn overlay(coords: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    let color = textureLoad(ray_casting, coords, 0);
    if (settings.overlay_mode == 0) {
        return color;
    }
    let delta = settings.central_difference_delta;
    let padding = delta * 2;
    if (coords.x < padding || coords.y < padding || coords.x >= size.x - padding || coords.y >= size.y - padding) {
        return color;
    }

    if (settings.overlay_mode == 1) {
        let exponent = lyapunov_exponent(coords);
        let scaled_exponent = exp(settings.lyapunov_scaling * exponent - 5.0);
        return scaled_exponent * vec4<f32>(1.0, 1.0, 1.0, 1.0) + (1.0 - scaled_exponent) * color;
    } elseif (settings.overlay_mode == 2) {
        if (ridge_test(coords, size)) {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
        return color;
    }
    return color;
}

[[stage(compute), workgroup_size(8, 8)]]
fn overlay_desktop([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y) - 1);
    if (coords.x >= size.x || coords.y < 0) {
        return;
    }
    let color = overlay(coords, size);
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
fn overlay_web([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y) - 1);
    if (coords.x >= size.x || coords.y < 0) {
        return;
    }
    let color = overlay(coords, size);
    textureStore(target, coords, vec4<f32>(srgb_from_linear(color.rgb), color.a));
}
