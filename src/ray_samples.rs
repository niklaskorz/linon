// edges of 3x3 grid (8 elements) with 100 samples per point
// Attributes:
// position: vec4<f32>
// color: vec4<f32>
// pub type RaySamples = [f32; 8 * 8 * 100];
const SAMPLES_PER_POINT: u16 = 100;

pub fn vertex_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    wgpu::VertexBufferLayout {
        array_stride: 32,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[
            // position: vec4<f32>
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x4,
            },
            // color: vec4<f32>
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    }
}

fn push_indices_for_side(result: &mut Vec<[u16; 3]>, a: u16, b: u16, c: u16) {
    for i in 0..(SAMPLES_PER_POINT - 1) {
        let a = a * SAMPLES_PER_POINT + i;
        let b = b * SAMPLES_PER_POINT + i;
        let c = c * SAMPLES_PER_POINT + i;

        result.push([a, b + 1, a + 1]);
        result.push([a, b, b + 1]);
        result.push([b, c, b + 1]);
        result.push([c, c + 1, b + 1]);
    }
}

pub fn create_indices() -> Vec<[u16; 3]> {
    let mut result: Vec<[u16; 3]> = vec![];
    push_indices_for_side(&mut result, 0, 1, 2);
    push_indices_for_side(&mut result, 2, 3, 4);
    push_indices_for_side(&mut result, 4, 5, 6);
    push_indices_for_side(&mut result, 6, 7, 0);
    result
}
