use std::{collections::HashMap, path::PathBuf, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use crossbeam::channel::{unbounded, Sender};
use engine_derive::GameEngine;
use to_from_bytes::{FromBytes, ToBytes};

use crate::{defaults::default_rendering::vectorinator::{Vectorinator, VectorinatorWrite}, horde::{game_engine::{engine::{GameEngine, MovingObjectID}, entity::{Entity, EntityID, EntityVec, MultiplayerEntity, Renderable}, multiplayer::{GlobalComponent, GlobalEvent, HordeEventReport, HordeMultiModeChoice, HordeMultiplayer, Identify, MultiplayerEngine, MustSync}, world::{World, WorldComputeHandler, WorldEvent, WorldHandler, WorldOutHandler, WorldWriteHandler}}, geometry::vec3d::Vec3Df, rendering::RenderingBackend, scheduler::IndividualTask, sound::{ARWWaves, SoundRequest, WaveIdentification, WavePosition, WaveRequest, WaveSink, WavesHandler}}, tests::entity_derive_test::CoolEntityVecWrite};

use super::{entity_derive_test::{CoolEntity, CoolEntityVecRead}, task_derive_test::SingleExtraData};
use to_from_bytes_derive::{FromBytes, ToBytes};

#[derive(Clone, ToBytes, FromBytes, PartialEq)]

pub struct SinglePWorld {
    pub test:usize,
}

impl<ID:Identify> WorldEvent<SinglePWorld, ID> for SinglePWorld {
    fn apply_event(self, world:&mut SinglePWorld) {
        
    }
    fn get_source(&self) -> Option<ID> {
        None
    }
    fn should_sync(&self) -> MustSync {
        MustSync::Both
    }
}

impl<ID:Identify> World<ID> for SinglePWorld {
    type RB = TestRB;
    type WE = SinglePWorld;
    fn update_rendering(&mut self, data:&mut <Self::RB as crate::horde::rendering::RenderingBackend>::PreTickData) {
        
    }
}

impl<'a> Renderable<VectorinatorWrite<'a>> for SinglePWorld {
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

pub fn after_main_tick<'a>(turn:EntityTurn, id:EntityID, reader: &CoolEntityVecRead<'a, SinglePEngineTID>, world_read: &WorldComputeHandler<'a, SinglePWorld, SinglePEngineTID>, waves:&SingleExtraData) {
    println!("TEST AFTER MAIN");
    match turn {
        EntityTurn::ent1 => {
            
        },
    }
    
}

pub fn compute_tick<'a>(turn:EntityTurn, id:EntityID, reader: &CoolEntityVecRead<'a, SinglePEngineTID>, world_read: &WorldComputeHandler<'a, SinglePWorld, SinglePEngineTID>, waves:&SingleExtraData) {
    if id == 0 {
        let tick = waves.tick.fetch_add(1, Ordering::Relaxed);
        println!("{}", tick);
        if tick == 0 {
            waves.waves_handler.request_sound(WaveRequest::Load(PathBuf::from("sounds/vine-boom.mp3")));
        }
        if tick >= 1 && tick % 300 == 0 {
            //waves.waves_handler.request_sound(WaveRequest::Sound(SoundRequest::new(WaveIdentification::ByName("vine-boom.mp3".to_string()), WavePosition::InsideYourHead, WaveSink::FirstEmpty)));
        }
    }
    println!("TEST COMPUTE");
}

#[derive(GameEngine, Clone)]
#[rendering_engine = "Vectorinator"]
pub struct SinglePEngine {
    ent1:CoolEntity,
    world:SinglePWorld,
    #[extra_data]
    extra_data:SingleExtraData
}