use std::{collections::HashMap, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use engine_derive::GameEngine;
use to_from_bytes::{FromBytes, ToBytes};

use crate::{defaults::default_rendering::vectorinator::{Vectorinator, VectorinatorWrite}, horde::{game_engine::{engine::{GameEngine, MovingObjectID}, entity::{Entity, EntityID, EntityVec, MultiplayerEntity, Renderable}, multiplayer::{GlobalComponent, GlobalEvent, HordeEventReport, HordeMultiModeChoice, HordeMultiplayer, Identify, MultiplayerEngine, MustSync}, world::{World, WorldComputeHandler, WorldEvent, WorldHandler, WorldOutHandler, WorldWriteHandler}}, geometry::vec3d::Vec3Df, rendering::RenderingBackend, scheduler::IndividualTask}, tests::entity_derive_test::CoolEntityVecWrite};

use super::entity_derive_test::{CoolEntity, CoolEntityVecRead};
use to_from_bytes_derive::{FromBytes, ToBytes};

#[derive(Clone, ToBytes, FromBytes, PartialEq)]

pub struct TestWorld {
    pub test:usize
}

impl<ID:Identify> WorldEvent<TestWorld, ID> for TestWorld {
    fn apply_event(self, world:&mut TestWorld) {
        
    }
    fn get_source(&self) -> Option<ID> {
        None
    }
    fn should_sync(&self) -> MustSync {
        MustSync::Both
    }
}

impl<ID:Identify> World<ID> for TestWorld {
    type RB = TestRB;
    type WE = TestWorld;
    fn update_rendering(&mut self, data:&mut <Self::RB as crate::horde::rendering::RenderingBackend>::PreTickData) {
        
    }
}

impl<'a> Renderable<VectorinatorWrite<'a>> for TestWorld {
    fn do_render_changes(&mut self, render_data:&mut VectorinatorWrite<'a>) {
        
    }
}

#[derive(Clone)]
pub struct TestRB {
    lol:usize
}

impl RenderingBackend for TestRB {
    type EntityRenderingData = usize;
    type PreTickData = usize;
    type RenderingStatus = usize;
    type RenderingStatusUpdate = usize;
}

pub fn after_main_tick<'a>(turn:EntityTurn, id:EntityID, reader: &CoolEntityVecRead<'a, TestEngineTID>, world_read: &WorldComputeHandler<'a, TestWorld, TestEngineTID>, extra_data:&usize) {
    println!("TEST AFTER MAIN");
    match turn {
        EntityTurn::ent1 => {
            
        },
    }
}

pub fn compute_tick<'a>(turn:EntityTurn, id:EntityID, reader: &CoolEntityVecRead<'a, TestEngineTID>, world_read: &WorldComputeHandler<'a, TestWorld, TestEngineTID>, extra_data:&usize) {
    println!("TEST COMPUTE");
}

#[derive(GameEngine)]
#[rendering_engine = "Vectorinator"]
#[do_multiplayer]
pub struct TestEngine {
    ent1:CoolEntity,
    world:TestWorld,
    #[extra_data]
    extra_data:usize
}