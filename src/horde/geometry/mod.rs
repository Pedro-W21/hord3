pub mod vec3d;
pub mod rotation;
pub mod plane;
pub mod line;
pub mod shapes_3d;

pub type HordeFloat = f32;

pub trait Intersection<T> {
    type IntersectionType;
    fn intersect_with(&self, target:&T) -> Self::IntersectionType;
}