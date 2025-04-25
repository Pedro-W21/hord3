use crate::horde::geometry::{rotation::{Orientation, Rotation}, vec3d::Vec3Df};

use super::{entity::Component, multiplayer::Identify};



pub trait EntityPosition<ID:Identify>:Component<ID> {
    fn get_pos(&self) -> Vec3Df;
    fn get_orientation(&self) -> Orientation;
    fn get_rotation(&self) -> Option<&Rotation>;
}