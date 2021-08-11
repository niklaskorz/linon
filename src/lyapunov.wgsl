// --- begin: translated from linalg.h

fn mat2det(a: mat2x2<f32>) -> f32 {
    return a.x.x * a.y.y - a.x.y * a.y.x;
}

fn mat2invariants(m: mat2x2<f32>) -> vec2<f32> {
    return vec2<f32>(
        mat2det(m),
        -(m.x.x + m.y.y)
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
var target: [[access(write)]] texture_storage_2d<rgba8unorm>;

[[stage(compute), workgroup_size(8, 8)]]
fn main([[builtin(global_invocation_id)]] gid: vec3<u32>) {
    let size = textureDimensions(target);
    let coords = vec2<i32>(i32(gid.x), size.y - i32(gid.y) - 1);
    if (coords.x >= size.x || coords.y < 0) {
        return;
    }
    let color = textureLoad(ray_casting, coords, 0);
    if (coords.x < 10 || coords.y < 10 || coords.x >= size.x - 10 || coords.y >= size.y - 10) {
        textureStore(target, coords, color);
        return;
    }
    let x_next = textureLoad(mapping, vec2<i32>(coords.x + 10, coords.y), 0).xyz;
    let x_prev = textureLoad(mapping, vec2<i32>(coords.x - 10, coords.y), 0).xyz;
    let y_next = textureLoad(mapping, vec2<i32>(coords.x, coords.y + 10), 0).xyz;
    let y_prev = textureLoad(mapping, vec2<i32>(coords.x, coords.y - 10), 0).xyz;
    let gradient = mat2x3<f32>(
        1.0/20.0 * (x_next - x_prev),
        1.0/20.0 * (y_next - y_prev)
    );
    let m = transpose(gradient) * gradient;
    let eigen = mat2eigenvalues(m);
    let exponent = sqrt(max(eigen[0], eigen[1]));
    let c = 10.0 * exponent;
    textureStore(target, coords, 0.5 * vec4<f32>(c, c, c, 1.0) + 0.5 * color);
    // textureStore(target, coords, vec4<f32>(c, c, c, 1.0));
}
