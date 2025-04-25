use std::{marker::PhantomData, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}, time::Duration};

use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::horde::rendering::RenderingBackend;

use super::multiplayer::Identify;

pub trait World<ID:Identify>: Sized + Sync + Send + Clone {
    type WE:WorldEvent<Self, ID>;
    type RB:RenderingBackend;
    fn update_rendering(&mut self, data:&mut <Self::RB as RenderingBackend>::PreTickData);
}

pub trait WorldEvent<W:World<ID>, ID:Identify>: Sized + Sync + Send + Clone {
    fn apply_event(self, world:&mut W);
    fn get_source(&self) -> Option<ID>;
    fn should_sync(&self) -> bool;
}

#[derive(Clone)]
pub struct WorldHandler<W: World<ID>, ID:Identify> {
    pub world: Arc<RwLock<W>>,
    pub tunnels_in: WorldTunnelsIn<W, ID>,
    pub tunnels_out: WorldTunnelsOut<W, ID>,
    marker:PhantomData<ID>,
}

impl<W: World<ID>, ID:Identify> WorldHandler<W, ID> {
    pub fn new(map: W) -> Self {
        let map_lock = Arc::new(RwLock::new(map));
        let tunnel_pair = WorldTunnelsIn::new(1);
        let other_lock = map_lock.clone();
        WorldHandler {
            world: map_lock,
            tunnels_in: tunnel_pair.0,
            tunnels_out: tunnel_pair.1,
            marker:PhantomData{},
        }
    }
    pub fn apply_all_events<'a>(&self, write_handler: &mut WorldWriteHandler<'a, W, ID>, multiplayer:Option<(RwLockWriteGuard<'a, Vec<W::WE>>, RwLockWriteGuard<'a, Vec<W::WE>>, bool)>) {
        match multiplayer {
            Some((mut writer,mut all_writer, is_server)) => {
                if is_server {
                    while let Ok(event) = self
                        .tunnels_in
                        .map_events
                        .recv_timeout(Duration::from_nanos(10))
                    {
                        if event.should_sync() {
                            writer.push(event.clone());
                        }
                        all_writer.push(event.clone());
                        event.apply_event(&mut write_handler.world);
                    }
                }
                else {
                    while let Ok(event) = self
                        .tunnels_in
                        .map_events
                        .recv_timeout(Duration::from_nanos(10))
                    {
                        if event.should_sync() {
                            writer.push(event.clone());
                        }
                        event.apply_event(&mut write_handler.world);
                    }
                }
                
            }
            None => {
                while let Ok(event) = self
                    .tunnels_in
                    .map_events
                    .recv_timeout(Duration::from_nanos(10))
                {
                    event.apply_event(&mut write_handler.world);
                }
            }
            
        }
        
    }
    pub fn reset_stop(&mut self) {
        self.tunnels_in.reset_stop();
    }
    pub fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
        self.tunnels_out.update_number_of_threads(number_of_threads, thread_number)
    }
}
#[derive(Clone)]
pub struct WorldTunnelsIn<W: World<ID>, ID:Identify> {
    map_events: Receiver<W::WE>,
    stop: WorldStop,
    stop_tunnel: Receiver<bool>,
    marker:PhantomData<ID>,
}

impl<W: World<ID>, ID:Identify> WorldTunnelsIn<W, ID> {
    pub fn new(number_of_threads: u8) -> (Self, WorldTunnelsOut<W, ID>) {
        let event_pair = unbounded();
        let stop_pair = unbounded();
        let stop = WorldStop::new(number_of_threads);
        (
            WorldTunnelsIn {
                map_events: event_pair.1,
                stop,
                stop_tunnel: stop_pair.1,
                marker:PhantomData
            },
            WorldTunnelsOut {
                map_events: event_pair.0,
                stop_tunnel: stop_pair.0,
                thread_number:0,
                number_of_threads:4,
                marker:PhantomData
            },
        )
    }
    pub fn update_number_of_threads(&mut self, number_of_threads: u8) {
        self.stop.number_of_threads = number_of_threads;
    } 
}
#[derive(Clone)]
pub struct WorldOutHandler<W: World<ID>, ID:Identify> {
    pub world: Arc<RwLock<W>>,
    pub tunnels: WorldTunnelsOut<W, ID>,
    marker:PhantomData<ID>,
}

impl<W:World<ID>, ID:Identify> WorldOutHandler<W, ID> {
    pub fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
        self.tunnels.update_number_of_threads(number_of_threads, thread_number)
    }
}
#[derive(Clone)]
pub struct WorldTunnelsOut<W: World<ID>, ID:Identify> {
    map_events: Sender<W::WE>,
    stop_tunnel: Sender<bool>,
    thread_number:usize,
    number_of_threads:usize,
    marker:PhantomData<ID>,
}

impl<W: World<ID>, ID:Identify> WorldTunnelsOut<W, ID> {
    pub fn send_stop(&self) {
        self.stop_tunnel.send(true);
    }
    pub fn send_event(&self, event: W::WE) {
        self.map_events.send(event);
    }
    pub fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
        self.number_of_threads = number_of_threads;
        self.thread_number = thread_number;
    }
}

#[derive(Clone)]
pub struct WorldStop {
    number_of_stops: u8,
    number_of_threads: u8,
}

impl WorldStop {
    fn new(number_of_threads: u8) -> Self {
        WorldStop {
            number_of_stops: 0,
            number_of_threads,
        }
    }
}

impl<W: World<ID>, ID:Identify> WorldTunnelsIn<W, ID> {
    pub fn handle_stop(&mut self, variant: bool) {
        if variant {
            self.stop.number_of_stops += 1;
        }
    }

    pub fn check_stops_if_calc_not_finished(&mut self) {
        if !self.calc_fini() {
            self.check_stops()
        }
    }

    pub fn check_stops(&mut self) {
        match self.stop_tunnel.try_recv() {
            Ok(variant) => self.handle_stop(variant),
            Err(_) => (),
        }
    }

    pub fn calc_fini(&self) -> bool {
        if self.stop.number_of_stops == self.stop.number_of_threads {
            true
        } else {
            false
        }
    }
    pub fn calc_fini_sound(&self) -> bool {
        if self.stop.number_of_stops == self.stop.number_of_threads + 1 {
            true
        } else {
            false
        }
    }
    pub fn reset_stop(&mut self) {
        self.stop.number_of_stops = 0;
    }
}

pub struct WorldWriteHandler<'a, W: World<ID>, ID:Identify> {
    pub world: RwLockWriteGuard<'a, W>,
    marker:PhantomData<ID>,
}

impl<'a,W: World<ID>, ID:Identify> WorldWriteHandler<'a, W, ID> {
    pub fn from_world_handler(handler: &'a WorldHandler<W, ID>) -> Self {
        Self {
            world: handler.world.write().unwrap(),
            marker:PhantomData
        }
    }
}

pub struct WorldComputeHandler<'a,W: World<ID>, ID:Identify> {
    pub world: RwLockReadGuard<'a, W>,
    pub tunnels: WorldTunnelsOut<W, ID>,
}

impl<'a, W: World<ID>, ID:Identify> WorldComputeHandler<'a, W, ID> {
    pub fn from_world_handler(handler: &'a WorldHandler<W, ID>) -> Self {
        Self {
            world: handler.world.read().unwrap(),
            tunnels: handler.tunnels_out.clone(),
        }
    }
}