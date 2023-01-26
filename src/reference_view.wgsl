struct Uniforms {
    camera_pos: vec4<f32>,
    view_projection: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

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

struct VertexInput {
    @builtin(vertex_index)
    vertex_index: u32,
};
struct VertexOutput {
    @builtin(position)
    clip_position: vec4<f32>,
    @location(0)
    position: vec3<f32>,
    @location(1)
    normal: vec3<f32>,
    @location(2)
    color: vec3<f32>,
};

@vertex
fn main_vertex(input: VertexInput) -> VertexOutput {
    let index = input.vertex_index / 3u;
    let face = faces.data[index];
    let a = vertices.data[face.a];
    let b = vertices.data[face.b];
    let c = vertices.data[face.c];
    var ttriangle: array<vec3<f32>, 3> = array<vec3<f32>, 3>(
        vec3<f32>(a.x, a.y, a.z),
        vec3<f32>(b.x, b.y, b.z),
        vec3<f32>(c.x, c.y, c.z),
    );
    let position = ttriangle[input.vertex_index % 3u];
    let d1 = ttriangle[1] - ttriangle[0];
    let d2 = ttriangle[2] - ttriangle[0];

    var output: VertexOutput;
    output.clip_position = uniforms.view_projection * vec4<f32>(position, 1.0);
    output.position = position;
    output.normal = normalize(cross(d1, d2));
    output.color = abs(output.normal);

    return output;
}

const light_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const ambient_strength: f32 = 0.01;
const shininess: f32 = 64.0;
const object_color: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);

fn srgb_from_linear(linear_rgb: vec3<f32>) -> vec3<f32> {
    // Based on https://gamedev.stackexchange.com/a/148088
    let cutoff = linear_rgb < vec3<f32>(0.0031308);
    let lower = linear_rgb / vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(linear_rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

fn main_fragment_shared(input: VertexOutput) -> vec3<f32> {
    let ambient = ambient_strength * light_color;
    // The camera is the light source here, which allows for
    // some simplifications
    let direction = normalize(input.position - uniforms.camera_pos.xyz);
    var intensity: f32 = max(dot(input.normal, -direction), 0.0);
    if (intensity == 0.0) {
        intensity = max(dot(-input.normal, -direction), 0.0);
    }
    let diff = intensity;
    let diffuse = diff * light_color;
    let spec = pow(intensity, shininess);
    let specular = spec * light_color;
    return (ambient + diffuse + specular) * input.color;
}

@fragment
fn main_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(main_fragment_shared(input), 1.0);
}

@fragment
fn main_fragment_web(input: VertexOutput) -> @location(0) vec4<f32> {
    let llinear = srgb_from_linear(main_fragment_shared(input));
    return vec4<f32>(llinear, 1.0);
}

struct SampleVertexInput {
    @location(0)
    position: vec4<f32>,
    @location(1)
    color: vec4<f32>,
};
struct SampleVertexOutput {
    @builtin(position)
    clip_position: vec4<f32>,
    @location(0)
    color: vec4<f32>,
};

@vertex
fn sample_vertex(input: SampleVertexInput) -> SampleVertexOutput {
    var output: SampleVertexOutput;
    output.clip_position = uniforms.view_projection * input.position;
    output.color = input.color;
    return output;
}

@fragment
fn sample_fragment(input: SampleVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@fragment
fn sample_fragment_web(input: SampleVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(srgb_from_linear(input.color.rgb), input.color.a);
}
