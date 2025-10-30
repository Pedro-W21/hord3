use super::{vec3d::Vec3Df};

pub enum DistanceResults {
    NotParallel(f32, f32),
    Parallel(f32, f32),
}

#[derive(Debug, Clone, Copy)]
pub struct Line3D {
    origin: Vec3Df,
    director: Vec3Df,
}

#[derive(Clone, Copy)]
pub struct LineCoef(pub f32);

impl Line3D {
    pub fn new(origin: Vec3Df, director: Vec3Df) -> Line3D {
        Line3D { origin, director }
    }
    pub fn get_origin(&self) -> &Vec3Df {
        &self.origin
    }
    pub fn get_director(&self) -> &Vec3Df {
        &self.director
    }
    pub fn get_point_at(&self, coef:LineCoef) -> Vec3Df {
        self.origin + self.director * coef.0 
    }
    pub fn calc_shortest_distance_coefs(&self, other: &Self) -> DistanceResults {
        let det = (self.director.norme_square() * other.director.norme_square())
            - self.director.dot(&other.director);
        if det.abs() > f32::EPSILON {
            let one_over_det = 1.0 / det;
            let diff_orig = other.origin - self.origin;
            let d1_d_AB = self.director.dot(&diff_orig);
            let d2_d_AB = other.director.dot(&diff_orig);
            let d1_d_d2 = self.director.dot(&other.director);
            let ns_d1 = self.director.norme_square();
            let ns_d2 = other.director.norme_square();

            let t2 = (d1_d_AB * d1_d_d2 - ns_d1 * d2_d_AB) * one_over_det;
            let t1 = (ns_d2 * d1_d_AB - d1_d_d2 * d2_d_AB) * one_over_det;

            DistanceResults::NotParallel(t1, t2)
        } else {
            DistanceResults::Parallel(
                (self.director.dot(&(other.origin - self.origin))) / self.director.norme_square(),
                0.0,
            )
        }
    }
    pub fn calc_shortest_distance_between_director_segments(&self, other: &Self) -> f32 {
        match self.calc_shortest_distance_coefs(other) {
            DistanceResults::NotParallel(t1, t2) => {
                (self.get_at(t1.clamp(0.0, 1.0)) - other.get_at(t2.clamp(0.0, 1.0))).norme()
            }
            DistanceResults::Parallel(t1, t2) => (self.get_at(t1) - other.get_at(t2)).norme(),
        }
    }
    pub fn get_at(&self, coef: f32) -> Vec3Df {
        self.origin + self.director * coef
    }
}