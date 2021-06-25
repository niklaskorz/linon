use cgmath::{Matrix4, SquareMatrix};

use crate::arcball::ArcballCamera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    origin: [f32; 4],
    view_direction: [f32; 4],
    up: [f32; 4],
    view_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn moving(camera: &ArcballCamera<f32>) -> CameraUniform {
        let eye_pos = camera.eye_pos();
        let eye_dir = camera.eye_dir();
        let up_dir = camera.up_dir();
        CameraUniform {
            origin: [eye_pos.x, eye_pos.y, eye_pos.z, 0.0],
            view_direction: [eye_dir.x, eye_dir.y, eye_dir.z, 0.0],
            up: [up_dir.x, up_dir.y, up_dir.z, 0.0],
            view_matrix: Matrix4::identity().into(),
        }
    }

    pub fn stationary(camera: &ArcballCamera<f32>) -> CameraUniform {
        CameraUniform {
            origin: [0.0, 0.0, 0.0, 0.0],
            view_direction: [0.0, 0.0, -1.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            view_matrix: camera.get_mat4().into(),
        }
    }
}
