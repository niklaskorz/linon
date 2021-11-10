//! Based on https://github.com/Twinklebear/arcball by Will Usher, MIT License.
//! An implementation of the [Shoemake Arcball Camera](https://www.talisman.org/~erlkonig/misc/shoemake92-arcball.pdf)
//! using [cgmath](https://crates.io/crates/cgmath). See the
//! [cube example](https://github.com/Twinklebear/arcball/blob/master/examples/cube.rs) for an example
//! of use with [glium](https://crates.io/crates/glium).

use cgmath::num_traits::clamp;
use cgmath::prelude::*;
use cgmath::{BaseFloat, Matrix4, Quaternion, Vector2, Vector3, Vector4};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CameraOperation {
    None,
    Rotate,
    Pan,
}

/// The Shoemake Arcball camera.
pub struct ArcballCamera<F> {
    pub center: Vector3<F>,
    translation: Matrix4<F>,
    center_translation: Matrix4<F>,
    rotation: Quaternion<F>,
    camera: Matrix4<F>,
    inv_camera: Matrix4<F>,
    zoom_speed: F,
    inv_screen: [F; 2],
}

impl<F: BaseFloat> ArcballCamera<F> {
    /// Create a new Arcball camera focused at the `center` point, which will zoom at `zoom_speed`
    /// `screen` should be `[screen_width, screen_height]`.
    pub fn new(center: Vector3<F>, zoom_speed: F, screen: [F; 2]) -> ArcballCamera<F> {
        let mut cam = ArcballCamera {
            center,
            translation: Matrix4::from_translation(Vector3::new(F::zero(), F::zero(), -F::one())),
            center_translation: Matrix4::from_translation(center).invert().unwrap(),
            rotation: Quaternion::new(F::one(), F::zero(), F::zero(), F::zero()),
            camera: Matrix4::from_scale(F::one()),
            inv_camera: Matrix4::from_scale(F::one()),
            zoom_speed,
            inv_screen: [F::one() / screen[0], F::one() / screen[1]],
        };
        cam.update_camera();
        cam
    }
    /// Get the view matrix computed by the camera.
    pub fn get_mat4(&self) -> Matrix4<F> {
        self.camera
    }
    /// Get the camera eye position
    pub fn eye_pos(&self) -> Vector3<F> {
        Vector3::new(
            self.inv_camera[3].x,
            self.inv_camera[3].y,
            self.inv_camera[3].z,
        )
    }
    /// Get the camera view direction
    pub fn eye_dir(&self) -> Vector3<F> {
        let dir = self.inv_camera * Vector4::new(F::zero(), F::zero(), -F::one(), F::zero());
        Vector3::new(dir.x, dir.y, dir.z).normalize()
    }
    /// Get the camera view direction
    pub fn up_dir(&self) -> Vector3<F> {
        let dir = self.inv_camera * Vector4::new(F::zero(), F::one(), F::zero(), F::zero());
        Vector3::new(dir.x, dir.y, dir.z).normalize()
    }
    /// Rotate the camera, mouse positions should be in pixel coordinates.
    ///
    /// Rotates from the orientation at the previous mouse position specified by `mouse_prev`
    /// to the orientation at the current mouse position, `mouse_cur`.
    pub fn rotate(&mut self, mouse_prev: Vector2<F>, mouse_cur: Vector2<F>) {
        let one = F::one();
        let two = F::from(2.0).unwrap();
        let m_cur = Vector2::new(
            clamp(mouse_cur.x * two * self.inv_screen[0] - one, -one, one),
            clamp(one - two * mouse_cur.y * self.inv_screen[1], -one, one),
        );
        let m_prev = Vector2::new(
            clamp(mouse_prev.x * two * self.inv_screen[0] - one, -one, one),
            clamp(one - two * mouse_prev.y * self.inv_screen[1], -one, one),
        );
        let mouse_cur_ball = ArcballCamera::screen_to_arcball(m_cur);
        let mouse_prev_ball = ArcballCamera::screen_to_arcball(m_prev);
        self.rotation = mouse_cur_ball * mouse_prev_ball * self.rotation;
        self.update_camera();
    }
    /// Zoom the camera in by some amount. Positive values zoom in, negative zoom out.
    pub fn zoom(&mut self, amount: F, elapsed: F) {
        let motion = Vector3::new(F::zero(), F::zero(), amount);
        self.translation =
            Matrix4::from_translation(motion * self.zoom_speed * elapsed) * self.translation;
        self.update_camera();
    }
    /// Pan the camera following the motion of the mouse. The mouse delta should be in pixels.
    pub fn pan(&mut self, mouse_delta: Vector2<F>) {
        let zoom_dist = self.translation[3][3].abs();
        let delta = Vector4::new(
            mouse_delta.x * self.inv_screen[0],
            -mouse_delta.y * self.inv_screen[1],
            F::zero(),
            F::zero(),
        ) * zoom_dist;
        let motion = self.inv_camera * delta;
        self.center_translation =
            Matrix4::from_translation(Vector3::new(motion.x, motion.y, motion.z))
                * self.center_translation;
        self.update_camera();
    }
    /// Update the screen dimensions, e.g. if the window has resized.
    pub fn update_screen(&mut self, width: F, height: F) {
        self.inv_screen[0] = F::one() / width;
        self.inv_screen[1] = F::one() / height;
    }
    fn update_camera(&mut self) {
        self.camera = self.translation * Matrix4::from(self.rotation) * self.center_translation;
        self.inv_camera = self.camera.invert().unwrap();
    }
    fn screen_to_arcball(p: Vector2<F>) -> Quaternion<F> {
        let dist = cgmath::dot(p, p);
        // If we're on/in the sphere return the point on it
        if dist <= F::one() {
            Quaternion::new(F::zero(), p.x, p.y, F::sqrt(F::one() - dist))
        } else {
            let unit_p = p.normalize();
            Quaternion::new(F::zero(), unit_p.x, unit_p.y, F::zero())
        }
    }
}
