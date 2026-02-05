

use std::f32::NAN;

use to_from_bytes_derive::{FromBytes, ToBytes};

use super::{line::{Line3D, LineCoef}, vec3d::Vec3Df, HordeFloat, Intersection};

pub struct VectorPlane {
    v1:Vec3Df,
    v2:Vec3Df,
    origin:Vec3Df
}

impl VectorPlane {
    pub fn new(v1:Vec3Df,v2:Vec3Df, origin:Vec3Df) -> Self {
        Self { v1, v2, origin }
    }
    pub fn directors(&self) -> (Vec3Df, Vec3Df) {
        (self.v1, self.v2)
    }
    pub fn origin(&self) -> Vec3Df {
        self.origin
    }
    pub fn get_normal(&self) -> Vec3Df {
        self.v1.cross(&self.v2)
    }
    pub fn to_equation_plane(&self) -> EquationPlane {
        let normal = self.v1.cross(&self.v2);
        let d = - (normal.x * self.origin.x + normal.y * self.origin.y + normal.z * self.origin.z);
        EquationPlane { normal, d }
    }
}


#[derive(Clone, ToBytes, FromBytes, Debug, PartialEq)]
pub struct EquationPlane {
    normal:Vec3Df,
    d:HordeFloat
}

impl EquationPlane {
    pub const fn new(normal:Vec3Df, d:f32) -> Self {
        Self { normal, d }
    }
    pub const fn get_normal(&self) -> Vec3Df {
        self.normal
    }
    pub fn is_point_in_plane(&self, point:&Vec3Df) -> bool {
        self.signed_distance(point).abs() < 0.001
    }
    pub fn signed_distance(&self, point:&Vec3Df) -> f32 {
        self.normal.dot(point) + self.d
    }
}

pub enum LinePlaneIntersection {
    Nothing,
    Point(LineCoef),
    Line,
}

impl LinePlaneIntersection {
    pub fn to_point(&self, line:&Line3D) -> Vec3Df {
        match self {
            LinePlaneIntersection::Line => line.get_point_at(LineCoef(1.0)),
            LinePlaneIntersection::Nothing => {line.get_point_at(LineCoef(NAN))},
            LinePlaneIntersection::Point(coef) => line.get_point_at(*coef)
        }
    }
    pub fn unwrap_coef(&self) -> f32 {
        match self {
            LinePlaneIntersection::Line => 1.0,
            LinePlaneIntersection::Nothing => NAN,
            LinePlaneIntersection::Point(coef) => coef.0
        }
    }
    pub fn is_something(&self) -> bool {
        match self {
            LinePlaneIntersection::Nothing => false,
            _ => true
        }
    }
}

impl Intersection<Line3D> for EquationPlane {
    type IntersectionType = LinePlaneIntersection;
    fn intersect_with(&self, target:&Line3D) -> Self::IntersectionType {
        let n_dot_dir = self.normal.dot(target.get_director());
        let perpendicular = n_dot_dir.abs() < 0.0001;
        if self.is_point_in_plane(target.get_origin()) && perpendicular {
            LinePlaneIntersection::Line
        }
        else if perpendicular {
            LinePlaneIntersection::Nothing
        }
        else {
            LinePlaneIntersection::Point(LineCoef((-self.d - self.normal.dot(target.get_origin()))/n_dot_dir))
        }
        
        
    }
}