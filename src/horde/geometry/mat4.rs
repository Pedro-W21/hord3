use crate::horde::geometry::{HordeFloat, vec3d::Vec3Df};

/// Almost fully taken directly from glam's implementation of the same methods with minor alterations to fit better here, just didn't want to add a big direct dependency if I could avoid it
pub struct Mat4 {
    /// Column-major
    data:[[HordeFloat ; 4] ; 4]
}

impl Mat4 {
    pub fn perspective(fov_y_radians: HordeFloat, aspect_ratio: HordeFloat, z_near: HordeFloat, z_far: HordeFloat) -> Self {
        let (sin_fov, cos_fov) = (0.5 * fov_y_radians).sin_cos();
        let h = cos_fov / sin_fov;
        let w = h / aspect_ratio;
        let r = z_far / (z_near - z_far);
        Self {
            data: [
                [w, 0.0, 0.0, 0.0],
                [0.0, h, 0.0, 0.0],
                [0.0, 0.0, r, -1.0],
                [0.0, 0.0, r * z_near, 0.0]
            ]
        }
    }


    pub fn look_to(eye: Vec3Df, dir: Vec3Df, up: Vec3Df) -> Self {
        let f = dir;
        let s = f.cross(&up).normalise();
        let u = s.cross(&f);

        Self {
            data: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-eye.dot(&s), -eye.dot(&u), eye.dot(&f), 1.0],
            ]
        }
    }

    pub fn look_at(eye: Vec3Df, center: Vec3Df, up: Vec3Df) -> Self {
        Self::look_to(eye, (center - eye).normalise(), up)
    }
    pub fn to_cols_array_2d(&self) -> [[f32 ; 4] ; 4] {
        self.data.clone()
    }
}
