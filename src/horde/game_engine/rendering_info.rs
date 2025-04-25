use crate::horde::rendering::RenderingBackend;

use super::{entity::Component, multiplayer::Identify};

pub trait RenderingInfo<RB:RenderingBackend, ID:Identify>:Component<ID> {
    fn add_update(&mut self, update:RB::RenderingStatusUpdate);
    fn get_status(&self) -> RB::RenderingStatus;
    fn get_data(&self) -> &RB::EntityRenderingData;
    fn get_data_mut(&mut self) -> &mut RB::EntityRenderingData;
}