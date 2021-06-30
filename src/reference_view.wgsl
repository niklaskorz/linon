[[block]]
struct Uniforms {
    camera_pos: vec4<f32>;
    view_projection: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

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

struct VertexInput {
    [[builtin(vertex_index)]]
    vertex_index: u32;
};
struct VertexOutput {
    [[builtin(position)]]
    clip_position: vec4<f32>;  
    [[location(0)]]
    position: vec3<f32>;
    [[location(1)]]
    normal: vec3<f32>;
    [[location(2)]]
    color: vec3<f32>;
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    let index = input.vertex_index / 3u;
    let face = faces.data[index];
    let a = vertices.data[face.a];
    let b = vertices.data[face.b];
    let c = vertices.data[face.c];
    var triangle: array<vec3<f32>, 3> = array<vec3<f32>, 3>(
        vec3<f32>(a.x, a.y, a.z),
        vec3<f32>(b.x, b.y, b.z),
        vec3<f32>(c.x, c.y, c.z),
    );
    let position = triangle[input.vertex_index % 3u];
    let d1 = triangle[1] - triangle[0];
    let d2 = triangle[2] - triangle[0];

    var output: VertexOutput;
    output.clip_position = uniforms.view_projection * vec4<f32>(position, 1.0);
    output.position = position;
    output.normal = normalize(cross(d1, d2));
    output.color = abs(output.normal);

    return output;
}

struct ArrowsVertexInput {
    [[location(0)]]
    position: vec3<f32>;
    [[location(1)]]
    normal: vec3<f32>;
    [[location(2)]]
    ray_position: vec4<f32>;
    [[location(3)]]
    ray_direction: vec4<f32>;
};

let arrow_base_dir: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);

[[stage(vertex)]]
fn arrows_main(input: ArrowsVertexInput) -> VertexOutput {
    var output: VertexOutput;
    if (input.ray_position.w < 0.0) {
        output.clip_position = uniforms.view_projection * vec4<f32>(0.0, 0.0, 0.0, -10000.0);
        return output;
    }

    let rot_axis = cross(arrow_base_dir, -input.ray_direction.xyz);
    let rot_cos = dot(arrow_base_dir, -input.ray_direction.xyz);
    let rot_sin = sin(acos(rot_cos));
    let rot_oc = 1.0 - rot_cos;
    let rot = mat4x4<f32>(
        vec4<f32>(rot_oc * rot_axis.x * rot_axis.x + rot_cos, rot_oc * rot_axis.x * rot_axis.y - rot_axis.z * rot_sin, rot_oc * rot_axis.z * rot_axis.x + rot_axis.y * rot_sin, 0.0),
        vec4<f32>(rot_oc * rot_axis.x * rot_axis.y + rot_axis.z * rot_sin, rot_oc * rot_axis.y * rot_axis.y + rot_cos, rot_oc * rot_axis.y * rot_axis.z - rot_axis.x * rot_sin, 0.0),
        vec4<f32>(rot_oc * rot_axis.z * rot_axis.x - rot_axis.y * rot_sin, rot_oc * rot_axis.y * rot_axis.z + rot_axis.x * rot_sin, rot_oc * rot_axis.z * rot_axis.z + rot_cos, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );

    let scaled = 0.05 * vec4<f32>(input.position, 1.0);
    let rotated = rot * scaled;
    let translated = rotated + input.ray_position;
    let position = translated.xyz;

    output.clip_position = uniforms.view_projection * vec4<f32>(position.xyz, 1.0);
    output.position = position;
    output.normal = input.normal;
    output.color = vec3<f32>(1.0, 1.0, 1.0);
    return output;
}

let light_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
let ambient_strength: f32 = 0.01;
let shininess: f32 = 64.0;
let object_color: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);
let use_lighting: bool = true;

[[stage(fragment)]]
fn main(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    if (!use_lighting) {
        return vec4<f32>(input.normal, 1.0);
    }
    let ambient = ambient_strength * light_color;
    // The camera is the light source here, which allows for
    // some simplifications
    let direction = normalize(input.position - uniforms.camera_pos.xyz);
    let intensity = max(dot(input.normal, -direction), 0.0);
    let diff = intensity;
    let diffuse = diff * light_color;
    let spec = pow(intensity, shininess);
    let specular = spec * light_color;
    let result = (ambient + diffuse + specular) * input.color;
    return vec4<f32>(result, 1.0);
}
