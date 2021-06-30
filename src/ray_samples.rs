use cgmath::{InnerSpace, Vector3};

// 10x10 grid with 100 samples per point
pub type RaySamples = [f32; 10 * 10 * 100];

#[rustfmt::skip]
const ARROW_GLYPH_VERTICES: [f32; 15] = [
    -0.25, 0.0, -0.25,
    0.25, 0.0, -0.25,
    0.25, 0.0, 0.25,
    -0.25, 0.0, 0.25,
    0.0, 1.0, 0.0,
];

#[rustfmt::skip]
const ARROW_GLYPH_INDICES: [usize; 18] = [
    // Square base
    0, 1, 2,
    2, 3, 0,
    // Triangle 1
    1, 0, 4,
    // Triangle 2
    2, 1, 4,
    // Triangle 3
    3, 2, 4,
    // Triangle 4
    0, 3, 4,
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ArrowGlyphVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl ArrowGlyphVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ArrowGlyphVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub fn create_allow_glyph() -> Vec<ArrowGlyphVertex> {
    let vertices = ARROW_GLYPH_VERTICES;
    let indices = ARROW_GLYPH_INDICES;
    let mut glyph: Vec<ArrowGlyphVertex> = vec![];

    let num_faces = indices.len() / 3;
    for i in 0..num_faces {
        let a_index = indices[i * 3];
        let a_vertex = a_index * 3;
        let a = Vector3::new(
            vertices[a_vertex],
            vertices[a_vertex + 1],
            vertices[a_vertex + 2],
        );
        let b_index = indices[i * 3 + 1];
        let b_vertex = b_index * 3;
        let b = Vector3::new(
            vertices[b_vertex],
            vertices[b_vertex + 1],
            vertices[b_vertex + 2],
        );
        let c_index = indices[i * 3 + 2];
        let c_vertex = c_index * 3;
        let c = Vector3::new(
            vertices[c_vertex],
            vertices[c_vertex + 1],
            vertices[c_vertex + 2],
        );
        let d1 = b - a;
        let d2 = c - a;
        let normal = d1.cross(d2).normalize();
        glyph.push(ArrowGlyphVertex {
            position: a.into(),
            normal: normal.into(),
        });
        glyph.push(ArrowGlyphVertex {
            position: b.into(),
            normal: normal.into(),
        });
        glyph.push(ArrowGlyphVertex {
            position: c.into(),
            normal: normal.into(),
        });
    }

    glyph
}
