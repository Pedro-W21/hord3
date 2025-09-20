use std::{collections::{HashMap, VecDeque}, hash::Hash, io::BufReader, path::PathBuf, sync::{Arc, RwLock}, thread, time::Duration};

use crossbeam::channel::{Sender, Receiver, unbounded};
use rodio::{Sink, Source, Sample, Decoder, OutputStreamHandle, OutputStream, SpatialSink};

use crate::horde::{game_engine::engine::MovingObjectID, geometry::rotation::Rotation};

use self::horde_source::{HordeSourceData, HordeSourceRaw};

use super::{game_engine::engine::GameEngine, geometry::vec3d::{Vec3D, Vec3Df}, rendering::camera::Camera, scheduler::IndividualTask};

pub mod horde_source;



pub struct SoundRequest<GE:GameEngine> {
    id:WaveIdentification,
    pos:WavePosition<GE>,
    sink:WaveSink,
}

impl<GE:GameEngine> SoundRequest<GE> {
    pub fn new(id:WaveIdentification, pos:WavePosition<GE>, sink:WaveSink) -> Self {
        Self { id, sink, pos }
    }
}

pub enum WavePosition<GE:GameEngine> {
    Fixed(Vec3Df),
    InsideYourHead,
    Moving(GE::MOID)
}

pub enum WaveRequest<GE:GameEngine> {
    Sound(SoundRequest<GE>),
    Load(PathBuf),
}

pub enum WaveSink {
    FirstEmpty,
    FirstNotEmpty,
    Precise(usize),
    PreciseForced(usize),
}

#[derive(Clone, Debug)]
pub enum WaveIdentification {
    ByName(String),
    ByID(usize)
}

#[derive(Clone)]
pub struct WavesHandler<GE:GameEngine> {
    request_send:Sender<WaveRequest<GE>>,
    compute_send:Sender<GE::GEC>,
    camera_time_send:Sender<(Camera, f32)>
}

impl<GE:GameEngine> WavesHandler<GE> {
    pub fn request_sound(&self, rqst:WaveRequest<GE>) {
        self.request_send.send(rqst);
    }
    pub fn send_gec(&self, gec:GE::GEC) {
        self.compute_send.send(gec);
    }
    pub fn send_camera_time(&self, cam:Camera, time:f32) {
        self.camera_time_send.send((cam, time));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WavePlay {
    id:usize,
    channel:usize,
}

pub struct Waves<GE:GameEngine> {
    audio_queue:VecDeque<WavePlay>,
    channels:Vec<Sink>,
    request_rcv:Receiver<WaveRequest<GE>>,
    gec_rcv:Receiver<GE::GEC>,
    camera_time_rcv:Receiver<(Camera, f32)>,
    tracks:Vec<HordeSourceData>,
    currently_noisy_things:HashMap<GE::MOID, Vec<usize>>,
    fixed_noisy_things:HashMap<Vec3Df, Vec<usize>>,
    stream_handle:OutputStreamHandle,
    compute_handler:Option<GE::GEC>,
    spatial_sinks:Vec<SpatialSink>,
    spatial_audio_queue:VecDeque<WavePlay>,
    left_ear:[f32 ; 3],
    right_ear:[f32 ; 3],
    
}

#[derive(Clone)]
pub struct ARWWaves<GE:GameEngine> {
    waves:Arc<RwLock<Waves<GE>>>
}

impl<GE:GameEngine + 'static> IndividualTask for ARWWaves<GE> {
    type TD = usize;
    type TID = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => self.waves.write().unwrap().do_dependent_updates(),
            1 => self.waves.write().unwrap().do_independent_updates(),
            _ => panic!("No other task IDs possible !!!")
        }
    }
}

impl<GE:GameEngine + 'static> Waves<GE> {
    pub fn new(tracks_to_load:Vec<PathBuf>, sinks:usize) -> (ARWWaves<GE>, WavesHandler<GE>, OutputStream) {
        
        let (send, receive) = unbounded();
        let (gec_send, gec_rcv) = unbounded();
        let (camera_time_send, camera_time_rcv) = unbounded();
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let mut channels = Vec::with_capacity(sinks);
        let mut spatial_sinks = Vec::with_capacity(sinks);
        for i in 0..sinks {
            channels.push(Sink::try_new(&stream_handle).unwrap());
            spatial_sinks.push(SpatialSink::try_new(&stream_handle, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]).unwrap());
        }
        
        let mut tracks = Vec::with_capacity(tracks_to_load.len());
        for track in tracks_to_load {
            tracks.push(HordeSourceData::new(HordeSourceRaw::from_file(track).unwrap()));
        }
        (ARWWaves {
        waves:Arc::new(RwLock::new(Waves {
            audio_queue:VecDeque::new(),
            channels,
            request_rcv:receive,
            tracks,
            stream_handle,
            compute_handler:None,
            gec_rcv,
            currently_noisy_things:HashMap::with_capacity(1000),
            fixed_noisy_things:HashMap::with_capacity(1000),
            camera_time_rcv,
            spatial_sinks,
            spatial_audio_queue:VecDeque::with_capacity(100),
            left_ear:[0.0 ; 3],
            right_ear:[0.0 ; 3]
        }))},


        WavesHandler { request_send: send, compute_send:gec_send, camera_time_send },
        stream)
    }
    pub fn new_just_handler() -> WavesHandler<GE> {
        WavesHandler { request_send: unbounded().0, compute_send:unbounded().0, camera_time_send:unbounded().0 }
    }
    pub fn wait_for_gec(&mut self) {
        self.compute_handler = Some(self.gec_rcv.recv().unwrap());
    }
    pub fn try_to_update_pos(&mut self) {
        if self.compute_handler.is_none() {
            self.wait_for_gec();
        }
        const EAR_DIST:f32 = 0.2;
        match self.camera_time_rcv.try_recv() {
            Ok((camera, time)) => {
                
                let camera_pos = camera.pos;
                let camera_orient = Rotation::new_from_euler(camera.orient.yaw, camera.orient.pitch, camera.orient.roll);
                let ear = camera_orient.rotate(Vec3D::new(1.0, 0.0, 0.0) * EAR_DIST);
                //dbg!(ear);
                self.right_ear = (camera_pos + ear).coords_to_array();
                self.left_ear = (camera_pos - ear).coords_to_array();
                //panic!("FLKQJSFLKQJSMFLKJQSFKJQMLFKJQMLSKFJMQLSKFJMQLKSJFMLQKJFMLQKJSFMLQKSJFMLQKJSFMLQKFMQLKJFMQLKSJFMLQKSJFMLQKSJFMLQKSJFMLKQJSFMLKQJSFMLKQSJFMLKSQMLFKJQSMLFKJQSMLKFJQMLSKFJQMLSKFJMQLSKFJMLQSKJFMQLKSJFMLQKSJFMLQKSJFMQLKSFJMQLSKJFMLQKSJFLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKSJFMLQKJFMLKQSJFMLKQSJFMLKQJSFMLKSJMFLKQSMFKLQJMSFLKJ");

                //println!("YAHOHAOHAOHOAHOAOHOAOHOOAHOOHAO");
                let moids:Vec<<GE as GameEngine>::MOID> = self.currently_noisy_things.keys().map(|moid| {moid.clone()}).collect();
                <<GE as GameEngine>::MOID as MovingObjectID<GE>>::for_each_position(&mut moids.into_iter(), 
                &mut |moid, pos| {
                    let moving_sinks = self.currently_noisy_things.get_mut(&moid).unwrap();
                    let mut remove_sinks = Vec::with_capacity(moving_sinks.len());
                    for (i, sink_id) in moving_sinks.iter().enumerate() {
                        let sink = &self.spatial_sinks[*sink_id];
                        if sink.empty() {
                            remove_sinks.push(i);
                        }
                        else {
                            sink.set_left_ear_position(self.left_ear);
                            sink.set_right_ear_position(self.right_ear);
                            sink.set_emitter_position(pos.coords_to_array());
                            //dbg!(pos.coords_to_array());
                        }
                    }
                    if remove_sinks.len() == moving_sinks.len() {
                        false
                    }
                    else {
                        remove_sinks.sort_unstable();
                        for sink in remove_sinks.iter().rev() {
                            moving_sinks.remove(*sink);
                        }
                        true
                    }
                }, self.compute_handler.as_ref().unwrap());
                
            }
            Err(_) => ()
        }
    }

    pub fn update_fixed_pos(&mut self) {
        self.fixed_noisy_things.retain(|pos, fixed_sinks|  {
            //dbg!(pos);
            let mut remove_sinks = Vec::with_capacity(fixed_sinks.len());
            for (i, sink_id) in fixed_sinks.iter().enumerate() {
                let sink = &self.spatial_sinks[*sink_id];
                if sink.empty() {
                    remove_sinks.push(i);
                }
                else {
                    sink.set_left_ear_position(self.left_ear);
                    sink.set_right_ear_position(self.right_ear);
                }
            }
            if remove_sinks.len() == fixed_sinks.len() {
                false
            }
            else {
                remove_sinks.sort_unstable();
                for sink in remove_sinks.iter().rev() {
                    fixed_sinks.remove(*sink);
                }
                true
            }
        });
    }
    pub fn do_independent_updates(&mut self) {
        if self.compute_handler.is_none() {
            self.wait_for_gec();
        }
        self.update_fixed_pos();
        while let Ok(request) = self.request_rcv.try_recv() {
            self.handle_request(request);
            while let Some(play) = self.audio_queue.pop_front() {
                //dbg!(play.clone());
                self.channels[play.channel].append(self.tracks[play.id].get_iter());
                //self.channels[play.channel].set_volume(100.0);
                //self.channels[play.channel].sleep_until_end();
            }
            while let Some(play) = self.spatial_audio_queue.pop_front() {
                //dbg!(play);
                self.spatial_sinks[play.channel].set_volume(100.0);
                self.spatial_sinks[play.channel].append(self.tracks[play.id].get_iter());
                //self.channels[play.channel].sleep_until_end();
            }
        }
    }
    pub fn do_dependent_updates(&mut self) {
        self.try_to_update_pos();
    }
    pub fn execution_loop(&mut self) {
        self.wait_for_gec();
        
        loop {
            
            self.try_to_update_pos();
            
            //println!("TESSSSSSSSSSSSSSSSSSSSTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT");
            while let Ok(request) = self.request_rcv.try_recv() {
                self.handle_request(request);
                while let Some(play) = self.audio_queue.pop_front() {
                    self.channels[play.channel].append(self.tracks[play.id].get_iter());
                    //self.channels[play.channel].sleep_until_end();
                }
                while let Some(play) = self.spatial_audio_queue.pop_front() {
                    //dbg!(play);
                    self.spatial_sinks[play.channel].set_volume(100.0);
                    self.spatial_sinks[play.channel].append(self.tracks[play.id].get_iter());
                    //self.channels[play.channel].sleep_until_end();
                }
            }
            
        }
    }
    pub fn handle_request(&mut self, request:WaveRequest<GE>) {
        match request {
            WaveRequest::Sound(sound_request) => self.handle_sound_request(sound_request),
            WaveRequest::Load(path) => match HordeSourceRaw::from_file(path) {
                Ok(raw) => self.tracks.push(HordeSourceData::new(raw)),
                Err(_) => println!("Couldn't Load ouin ouin"),
            }
        }
    }
    pub fn handle_sound_request(&mut self, request:SoundRequest<GE>) {
        // dbg!(request.id.clone());
        match request.id {
            WaveIdentification::ByID(id) => if id < self.tracks.len() {
                self.add_play(id, request.sink, request.pos)
            },
            WaveIdentification::ByName(name) => match self.tracks.iter().enumerate().find(|(i, track)| {track.get_name() == name}) {
                Some((i, track)) => self.add_play(i, request.sink, request.pos),
                None => (),
            }
        }
    }
    pub fn add_play_in_head(&mut self, id:usize, sink:WaveSink) {
        //panic!("{}", id);
        let channel = match sink {
            WaveSink::FirstEmpty => match self.channels.iter().enumerate().find(|(i, sink)| {sink.empty()}) {
                Some((i, _)) => i,
                None => {
                    self.channels.push(Sink::try_new(&self.stream_handle).unwrap());
                    self.channels.len() - 1
                }
            },
            WaveSink::FirstNotEmpty => match self.channels.iter().enumerate().find(|(i, sink)| {!sink.empty()}) {
                Some((i, _)) => i,
                None => {
                    self.channels.push(Sink::try_new(&self.stream_handle).unwrap());
                    self.channels.len() - 1
                }
            },
            WaveSink::Precise(sink_id) => if sink_id < self.channels.len() {
                sink_id
            }
            else {
                0
            },
            WaveSink::PreciseForced(sink_id) => if sink_id < self.channels.len() {
                sink_id
            }
            else {
                for i in self.channels.len()..sink_id + 1 {
                    self.channels.push(Sink::try_new(&self.stream_handle).unwrap());
                }
                self.channels.len() - 1
            }
        };
        self.audio_queue.push_back(WavePlay { id, channel })
    }

    pub fn add_play_fixed(&mut self, id:usize, sink:WaveSink, pos:Vec3Df) {
        let channel = match sink {
            WaveSink::FirstEmpty => match self.spatial_sinks.iter().enumerate().find(|(i, sink)| {sink.empty()}) {
                Some((i, _)) => {
                    self.spatial_sinks[i].set_emitter_position(pos.coords_to_array());
                    i
                },
                None => {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                    self.spatial_sinks.len() - 1
                }
            },
            WaveSink::FirstNotEmpty => match self.spatial_sinks.iter().enumerate().find(|(i, sink)| {!sink.empty()}) {
                Some((i, _)) => {
                    self.spatial_sinks[i].set_emitter_position(pos.coords_to_array());
                    i
                },
                None => {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                    self.spatial_sinks.len() - 1
                }
            },
            WaveSink::Precise(sink_id) => if sink_id < self.spatial_sinks.len() {
                self.spatial_sinks[sink_id].set_emitter_position(pos.coords_to_array());
                sink_id
            }
            else {
                0
            },
            WaveSink::PreciseForced(sink_id) => if sink_id < self.spatial_sinks.len() {
                self.spatial_sinks[sink_id].set_emitter_position(pos.coords_to_array());
                sink_id
            }
            else {
                for i in self.spatial_sinks.len()..sink_id + 1 {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                }
                self.spatial_sinks[sink_id].set_emitter_position(pos.coords_to_array());
                self.spatial_sinks.len() - 1
            }
        };
        match self.fixed_noisy_things.get_mut(&pos) {
            Some(sinks) => if !sinks.contains(&channel) {
                sinks.push(channel);
            },
            None => {self.fixed_noisy_things.insert(pos, vec![channel]);}
        }
        self.spatial_audio_queue.push_back(WavePlay { id, channel })
    }

    pub fn add_play_moving(&mut self, id:usize, sink:WaveSink, moid:GE::MOID) {
        let pos = moid.get_position(&self.compute_handler.as_ref().unwrap());
        let channel = match sink {
            WaveSink::FirstEmpty => match self.spatial_sinks.iter().enumerate().find(|(i, sink)| {sink.empty()}) {
                Some((i, _)) => i,
                None => {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                    self.spatial_sinks.len() - 1
                }
            },
            WaveSink::FirstNotEmpty => match self.spatial_sinks.iter().enumerate().find(|(i, sink)| {!sink.empty()}) {
                Some((i, _)) => i,
                None => {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                    self.spatial_sinks.len() - 1
                }
            },
            WaveSink::Precise(sink_id) => if sink_id < self.spatial_sinks.len() {
                sink_id
            }
            else {
                0
            },
            WaveSink::PreciseForced(sink_id) => if sink_id < self.spatial_sinks.len() {
                sink_id
            }
            else {
                for i in self.spatial_sinks.len()..sink_id + 1 {
                    self.spatial_sinks.push(SpatialSink::try_new(&self.stream_handle, pos.coords_to_array(), self.left_ear, self.right_ear).unwrap());
                }
                self.spatial_sinks.len() - 1
            }
        };
        match self.currently_noisy_things.get_mut(&moid) {
            Some(sinks) => if !sinks.contains(&channel) {
                sinks.push(channel);
            },
            None => {self.currently_noisy_things.insert(moid, vec![channel]);}
        }
        self.spatial_audio_queue.push_back(WavePlay { id, channel })
    }

    pub fn add_play(&mut self, id:usize, sink:WaveSink, pos:WavePosition<GE>) {
        match pos {
            WavePosition::InsideYourHead => self.add_play_in_head(id, sink),
            WavePosition::Fixed(pos) => self.add_play_fixed(id, sink, pos),
            WavePosition::Moving(moid) => self.add_play_moving(id, sink, moid)
        }
    }
}