use std::hash::Hash;

use crate::horde::geometry::vec3d::Vec3Df;

pub trait GameEngine:Sync + Send + Sized {
    type GEC:Sync + Send + Sized;
    type MOID: MovingObjectID<Self>;
}

pub trait MovingObjectID<GE:GameEngine>:Send + Sync + Sized + Eq + PartialEq + Hash + Clone {
    fn get_position(&self, compute:&GE::GEC) -> Vec3Df;
    fn for_each_position<I:Iterator<Item = Self>, T, D:FnMut(Self, Vec3Df) -> T>(moids:&mut I, do_func:&mut D, compute:&GE::GEC) -> Vec<T>;
}