use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::horde::utils::parallel_counter::ParallelCounter;

use super::multiplayer::Identify;

pub trait Entity<ID:Identify>:Sized + Sync + Send {
    type EV<O>:EntityVec<ID>;
    type SE:StaticEntity<ID>;
    type NE:NewEntity<Self, ID>;
}

pub trait MultiplayerEntity<TID:Identify> {
    type ID;
    type GEV<O>;
    type GEC;
}

pub trait StaticEntity<ID:Identify> {

}

pub type EntityID = usize;

pub trait EntityVec<ID:Identify>:Send + Sync + Sized + Clone {
    type OutVec;
}

pub trait Component<ID:Identify>:Send + Sync + Sized + Clone {
    type SC:StaticComponent;
    fn from_static(static_comp:&Self::SC) -> Self;
    type CE:ComponentEvent<Self, ID>;
}

pub trait NewEntity<E:Entity<ID>, ID:Identify>:Sized + Sync + Send {
    fn get_ent(self, static_type:&E::SE) -> E;
}

pub trait Renderable<RB>:Sized + Sync + Send {
    fn do_render_changes(&mut self, render_data:&mut RB);
}

pub trait ComponentEvent<C:Component<ID>, ID:Identify>: Send + Sync + Clone {
    type ComponentUpdate;
    fn get_id(&self) -> EntityID;
    fn apply_to_component(self, components:&mut Vec<C>);
    fn get_source(&self) -> Option<ID>;
}

pub trait StaticComponent:Send + Sync + Sized + Clone {

}

#[derive(Clone)]
pub struct EVecStopsIn {
    stops_recv:Receiver<usize>,
    stops_counter:usize,
    target_count:usize,
    thread_number:usize,
    pub iteration_counter:ParallelCounter,
}

impl EVecStopsIn {
    pub fn new() -> (Self, EVecStopsOut) {
        let (stops_send, stops_recv) = unbounded();
        let counter = ParallelCounter::new(0, 64);
        (
            EVecStopsIn {
                stops_recv,
                stops_counter:0,
                target_count:4,
                thread_number:0,
                iteration_counter:counter.clone()
            },
            EVecStopsOut {
                stops_send,
                number_of_threads:4,
                thread_number:0,
                iteration_counter:counter
            }
        )
    }
    pub fn calc_fini(&self) -> bool {
        self.stops_counter >= self.target_count
    }
    pub fn reset_stop(&mut self, new_target:Option<usize>) {
        self.stops_counter = 0;
        match new_target {
            Some(trgt) => self.target_count = trgt,
            None => ()
        }
    }
    pub fn update_number_of_threads(&mut self, new_num:usize, thread_number:usize) {
        self.target_count = new_num;
        self.thread_number = thread_number;
    }
    pub fn check_stops(&mut self) {
        match self.stops_recv.recv() {
            Ok(variant) => self.stops_counter += 1,
            Err(err) => println!("{}", err)
        }
    }
    pub fn check_stops_if_calc_not_finished(&mut self) {
        if !self.calc_fini() {
            self.check_stops()
        }
    }

}
#[derive(Clone)]
pub struct EVecStopsOut {
    stops_send:Sender<usize>,
    number_of_threads:usize,
    thread_number:usize,
    pub iteration_counter:ParallelCounter
}

impl EVecStopsOut {
    pub fn send_stop(&self) {
        self.stops_send.send(1);
    }
    pub fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
        self.number_of_threads = number_of_threads;
        self.thread_number = thread_number;
    }
}