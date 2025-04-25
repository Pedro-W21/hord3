use super::{entity::Component, multiplayer::Identify};

pub trait Status<ID:Identify>:Component<ID> {
    fn is_alive(&self) -> bool;
}