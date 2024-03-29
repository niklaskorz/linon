@group(0) @binding(0)
var ttarget: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var mapping: texture_storage_2d<rgba32float, write>;

struct Camera {
    origin: vec4<f32>,
    view_direction: vec4<f32>,
    up: vec4<f32>,
    view_matrix: mat4x4<f32>,
}
@group(0) @binding(2)
var<uniform> camera: Camera;

struct Settings {
    field_weight: f32,
    mouse_pos_x: f32,
    mouse_pos_y: f32,
    overlay_mode: i32,
    central_difference_delta: i32,
    lyapunov_scaling: f32,
};
@group(0) @binding(3)
var<uniform> settings: Settings;

struct Exponents {
    data: array<f32>,
};
@group(0) @binding(4)
var<storage, read> exponents: Exponents;

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
};
struct Vertices {
    data: array<Vertex>,
};
@group(1) @binding(0)
var<storage, read> vertices: Vertices;

struct Face {
    a: u32,
    b: u32,
    c: u32,
};
struct Faces {
    data: array<Face>,
};
@group(1) @binding(1)
var<storage, read> faces: Faces;

struct RaySample {
    position: vec4<f32>,
    color: vec4<f32>,
};
struct RaySamples {
    data: array<RaySample, 800>,
};
@group(2) @binding(0)
var<storage, read_write> ray_samples: RaySamples;

const backface_culling: bool = false;

const light_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const ambient_strength: f32 = 0.01;
const shininess: f32 = 64.0;
const object_color: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);

const linear_mode: bool = false;
const use_lighting: bool = true;
const eps: f32 = 0.0000001;

const PI: f32 = 3.141592653589793;

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

// t: temperature in Celsius
fn refraction_index(t: f32) -> f32 {
    // Calculation term by Y. Zhao et al
    let air_pressure = 101325.0; // Pascal (nominal air pressure at 15 degrees Celsius sea level)
    let c1 = 0.0000104;
    let c2 = 0.00366;
    return c1 * air_pressure * (1.0 + air_pressure * (60.1 - 0.972 * t) * pow(10.0, -10.0)) / (1.0 + c2 * t);
}

fn refraction(t_in: f32, t_out: f32, v_in: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let eta_in = refraction_index(t_in);
    let eta_out = refraction_index(t_out);
    var cosi: f32 = clamp(-1.0, 1.0, dot(v_in, n));
    var n_ref: vec3<f32> = n;
    if (cosi < 0.0) {
        cosi = -cosi;
    } else {
        n_ref = -n;
    }
    let eta = eta_in / eta_out;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
    if (k < 0.0) {
        // total reflection
        return reflect(v_in, n_ref);
    }
    return eta * v_in + (eta * cosi - sqrt(k)) * n_ref;
}

fn point_plane_distance(p: vec3<f32>, n: vec3<f32>, p0: vec3<f32>) -> f32 {
    let d = dot(p0, n);
    // assuming n is a unit vector
    return abs(dot(p, n) - d);
}

fn sigmoid(x: f32) -> f32 {
    return 1.0 / (1.0 + exp(-x));
}

fn field_function(p_prev: vec3<f32>, p: vec3<f32>, v0: vec3<f32>, v: vec3<f32>, t: f32) -> vec3<f32> { return v; }

fn hit_triangle(v_in: array<vec3<f32>, 3>, origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    // Moeller-Trumbore intersection algorithm
    let edge1 = v_in[1] - v_in[0];
    let edge2 = v_in[2] - v_in[0];
    let h = cross(direction, edge2);
    let a = dot(edge1, h);
    if ((backface_culling || a > -eps) && a < eps) {
        return -1.0;
    }
    let f = 1.0 / a;
    let s = origin - v_in[0];
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
        let ttriangle = array<vec3<f32>, 3>(
            vec3<f32>(a.x, a.y, a.z),
            vec3<f32>(b.x, b.y, b.z),
            vec3<f32>(c.x, c.y, c.z),
        );
        t_new = hit_triangle(ttriangle, origin, direction);
        if (t_new > 0.0 && t_new < max_dist && (t < 0.0 || t_new < t)) {
            t = t_new;
            d1 = ttriangle[1] - ttriangle[0];
            d2 = ttriangle[2] - ttriangle[0];
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
        var intensity: f32 = max(dot(normal, -direction), 0.0);
        if (intensity == 0.0) {
            intensity = max(dot(-normal, -direction), 0.0);
        }
        let diff = intensity;
        let diffuse = diff * light_color;
        let spec = pow(intensity, shininess);
        let specular = spec * light_color;
        let result = (ambient + diffuse + specular) * abs(normal);
        return vec4<f32>(result, t);
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

struct NonlinearRayColorResult {
    color: vec4<f32>,
    mapping_point: vec4<f32>,
};

const adaptive_sampling: bool = true;

fn nonlinear_ray_color(start_point: vec3<f32>, start_dir: vec3<f32>) -> NonlinearRayColorResult {
    var result: NonlinearRayColorResult;
    result.mapping_point = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    let field_weight = settings.field_weight;
    var has_color: bool = false;
    var has_mapping_point: bool = false;
    var cur_point: vec3<f32> = start_point;
    var cur_dir: vec3<f32> = start_dir;
    var t: f32 = 0.0;
    var last_v: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var last_diff: f32 = -1.0;
    let h_initial = 0.1;
    var h: f32 = h_initial;

    for (; t <= 5.0;) {
        // Runge-Kutta method
        let k1 = field_function(cur_point, cur_point, start_dir, cur_dir, t);
        let k2 = field_function(cur_point, cur_point + 0.5 * h * k1, start_dir, k1, t + 0.5 * h);
        let k3 = field_function(cur_point, cur_point + 0.5 * h * k2, start_dir, k2, t + 0.5 * h);
        let k4 = field_function(cur_point, cur_point + h * k3, start_dir, k3, t + h);
        let v = (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
        let diff = length(v - last_v);
        if (adaptive_sampling && last_diff >= 0.0 && diff > 10.0 * last_diff && h > 0.002) {
            h = 0.001;
            continue;
        }
        cur_dir = (1.0 - field_weight) * cur_dir + field_weight * v;

        let step_dir = cur_dir * h;
        if (!has_color) {
            result.color = ray_color(cur_point, normalize(step_dir), length(step_dir));
            has_color = result.color.a > 0.0;
        }

        cur_point = cur_point + step_dir;
        t = t + h;
        last_v = v;
        if (2.0 * diff < last_diff && h < h_initial) {
            h = h_initial;
        }
        last_diff = diff;
    }

    result.mapping_point = vec4<f32>(cur_point, 1.0);
    result.color.a = 1.0;
    if (!has_color) {
        result.color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return result;
}

fn sample_rays(start_point: vec3<f32>, start_dir: vec3<f32>, samples_index: i32, sample_color: vec3<f32>) {
    let field_weight = settings.field_weight;
    var cur_point: vec3<f32> = start_point;
    var cur_dir: vec3<f32> = start_dir;
    var color: vec4<f32>;
    var t: f32 = 0.0;
    let h = 0.001;
    let steps: i32 = 5000;

    var ssample: RaySample;
    ssample.color = vec4<f32>(sample_color, 0.5);
    let sample_steps = 100;
    let sample_step_size = steps / sample_steps;

    for (var i: i32 = 0; i < steps; i = i + 1) {
        // Runge-Kutta method
        let k1 = field_function(cur_point, cur_point, start_dir, cur_dir, t);
        let k2 = field_function(cur_point, cur_point + 0.5 * h * k1, start_dir, k1, t + 0.5 * h);
        let k3 = field_function(cur_point, cur_point + 0.5 * h * k2, start_dir, k2, t + 0.5 * h);
        let k4 = field_function(cur_point, cur_point + h * k3, start_dir, k3, t + h);
        let v = (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
        cur_dir = (1.0 - field_weight) * cur_dir + field_weight * v;

        let step_dir = cur_dir * h;
        if (i % sample_step_size == 0) {
            ssample.position = vec4<f32>(cur_point, 1.0);
            let index = samples_index * sample_steps + i / sample_step_size;
            ray_samples.data[index] = ssample;
        }

        cur_point = cur_point + step_dir;
        t = t + h;
    }
}

var<private> sample_positions: array<vec2<f32>, 8> = array<vec2<f32>, 8>(
    vec2<f32>(-1.0, -1.0), // 0: (0, 0)
    vec2<f32>(0.0, -1.0), // 1: (1, 0)
    vec2<f32>(1.0, -1.0), // 2: (2, 0)
    vec2<f32>(1.0, 0.0), // 3: (2, 1)
    vec2<f32>(1.0, 1.0), // 4: (2, 2)
    vec2<f32>(0.0, 1.0), // 5: (1, 2)
    vec2<f32>(-1.0, 1.0), // 6: (0, 2)
    vec2<f32>(-1.0, 0.0), // 7: (1, 2)
);
var<private> sample_colors: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.5, 0.5, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.5, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
    vec3<f32>(0.5, 0.5, 0.5),
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(0.5, 0.0, 0.5),                           
);

const sample_outline_rays: bool = false;

@compute @workgroup_size(8, 8)
fn main_view(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(ttarget);
    let coords = vec2<i32>(i32(gid.x), i32(size.y) - i32(gid.y) - 1);
    if (coords.x >= i32(size.x) || coords.y < 0) {
        return;
    }

    let width = f32(i32(size.x));
    let height = f32(i32(size.y));
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

    if (linear_mode) {
        let color = ray_color(origin, dir, 100.0);
        textureStore(ttarget, coords, color);
    } else {
        let result = nonlinear_ray_color(origin, dir);
        textureStore(ttarget, coords, result.color);
        textureStore(mapping, coords, result.mapping_point);
    }

    // Sample points for reference view
    // Only executed in first workgroup for best performance
    if (gid.x < 8u && gid.y == 0u) {
        var pos: vec2<f32> = vec2<f32>(0.0, 0.0);
        if (sample_outline_rays) {
            let min_exp = 0.8;
            var found: bool = false;
            var sum: i32 = 0;
            if (gid.x == 0u) {
                // Bottom left
                for (var x: i32 = 0; x < i32(size.x); x = x + 1) {
                    for (var y: i32 = 0; y < i32(size.y); y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp && (i32(size.x) - x) + y >= sum) {
                            pos = vec2<f32>(f32(x), f32(y));
                            sum = (i32(size.x) - x) + y;
                        }
                    }
                }
            } else if (gid.x == 1u) {
                // Bottom middle
                for (var y: i32 = i32(size.x) - 1; y >= 0 && !found; y = y - 1) {
                    for (var x: i32 = 0; x < i32(size.x) && !found; x = x + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp) {
                            pos = vec2<f32>(f32(x), f32(y));
                            found = true;
                        }
                    }
                }
            } else if (gid.x == 2u) {
                // Bottom right
                for (var x: i32 = 0; x < i32(size.x); x = x + 1) {
                    for (var y: i32 = 0; y < i32(size.y); y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp && x + y >= sum) {
                            pos = vec2<f32>(f32(x), f32(y));
                            sum = x + y;
                        }
                    }
                }
            } else if (gid.x == 3u) {
                // Middle right
                for (var x: i32 = i32(size.x) - 1; x >= 0 && !found; x = x - 1) {
                    for (var y: i32 = 0; y < i32(size.y) && !found; y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp) {
                            pos = vec2<f32>(f32(x), f32(y));
                            found = true;
                        }
                    }
                }
            } else if (gid.x == 4u) {
                // Top right
                for (var x: i32 = 0; x < i32(size.x); x = x + 1) {
                    for (var y: i32 = 0; y < i32(size.y); y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp && x + (i32(size.y) - y) >= sum) {
                            pos = vec2<f32>(f32(x), f32(y));
                            sum = x + (i32(size.y) - y);
                        }
                    }
                }
            } else if (gid.x == 5u) {
                // Top middle
                for (var y: i32 = 0; y < i32(size.y) && !found; y = y + 1) {
                    for (var x: i32 = 0; x < i32(size.x) && !found; x = x + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp) {
                            pos = vec2<f32>(f32(x), f32(y));
                            found = true;
                        }
                    }
                }
            } else if (gid.x == 6u) {
                // Top left
                for (var x: i32 = 0; x < i32(size.x); x = x + 1) {
                    for (var y: i32 = 0; y < i32(size.y); y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp && (i32(size.x) - x) + (i32(size.y) - y) >= sum) {
                            pos = vec2<f32>(f32(x), f32(y));
                            sum = (i32(size.x) - x) + (i32(size.y) - y);
                        }
                    }
                }
            } else if (gid.x == 7u) {
                // Middle left
                for (var x: i32 = 0; x < i32(size.x) && !found; x = x + 1) {
                    for (var y: i32 = 0; y < i32(size.y) && !found; y = y + 1) {
                        if (exponents.data[y * i32(size.y) + x] >= min_exp) {
                            pos = vec2<f32>(f32(x), f32(y));
                            found = true;
                        }
                    }
                }
            }

            pos.x = pos.x / (width - 1.0);
            pos.y = (height - 1.0 - pos.y) / (height - 1.0);
        } else {
            let mouse_pos = vec2<f32>(settings.mouse_pos_x, settings.mouse_pos_y);
            pos = mouse_pos + 0.01 * sample_positions[i32(gid.x)];
        }

        let color = sample_colors[i32(gid.x)];
        let u2 = pos.x * viewport_width - 0.5 * viewport_width;
        let v2 = pos.y * viewport_height - 0.5 * viewport_height;
        let s2 = u2 * normalize(horizontal) + v2 * normalize(vertical) + focal_length * view_direction;
        let dir2 = normalize(s2);
        sample_rays(origin, dir2, coords.x, color);
    }
}