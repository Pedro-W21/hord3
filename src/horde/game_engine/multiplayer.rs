use std::{collections::{HashMap, HashSet}, hash::Hash, io::{self, Read, Write}, net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs}, ops::Mul, sync::{atomic::{AtomicUsize, Ordering}, Arc, RwLock}, time::Duration};

use crossbeam::channel::{unbounded, Receiver, Sender};
use to_from_bytes::{decode_from_tcp, ByteDecoderUtilities, FromBytes, ToBytes};
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::horde::utils::parallel_counter::ParallelCounter;

pub trait Identify:Clone + Hash + Sync + Send + ToBytes + FromBytes + Eq + PartialEq {

}

pub trait GlobalEvent:Clone + Sync + Send + ToBytes + FromBytes + PartialEq {
    type GC:GlobalComponent;
    type WD:Clone + Sync + Send + ToBytes + FromBytes;
}


pub trait MultiplayerEngine:Clone {
    type GE:GlobalEvent;
    type ID:Identify;
    type RIDG;
    fn get_all_components_and_world(&self) -> (Vec<(Self::ID, <Self::GE as GlobalEvent>::GC)>, <Self::GE as GlobalEvent>::WD);
    fn set_component(&mut self, id:Self::ID, value:<Self::GE as GlobalEvent>::GC);
    fn set_world(&mut self, world:<Self::GE as GlobalEvent>::WD);
    fn is_that_component_correct(&self, id:&Self::ID, component:&<Self::GE as GlobalEvent>::GC) -> bool;
    fn get_event_origin(&self, event:&Self::GE) -> Option<Self::ID>;
    fn get_tick(&self, event:&Self::GE) -> usize;
    fn get_target(event:&Self::GE) -> Self::ID;
    fn get_components_to_sync_for(&self, id:&Self::ID) -> Vec<<Self::GE as GlobalEvent>::GC>;
    fn apply_event(&mut self, event:Self::GE);
    fn generate_random_id(&self, generator:&mut Self::RIDG) -> Option<Self::ID>;
    fn get_random_id_generator() -> Self::RIDG;
    fn get_latest_event_report(&self) -> Option<HordeEventReport<Self::ID, Self::GE>>;
}

pub trait GlobalComponent: Clone + Sync + Send + ToBytes + FromBytes + PartialEq {

}

#[derive(Clone, ToBytes, FromBytes)]
pub enum HordeMultiplayerPacket<ME:MultiplayerEngine> {
    PlayerJoined(HordePlayer<ME::ID>),
    WannaJoin(String), // player name
    ThatsUrPlayerID(usize, usize), // Player id, tickrate
    SpreadEvent(ME::GE),
    DoYouAgree(ME::GE),
    DoYouAgreeComponent(ME::ID, <ME::GE as GlobalEvent>::GC),
    Chat{from_player:usize, text:String},
    ResetComponent{id:ME::ID, data:<ME::GE as GlobalEvent>::GC},
    ResetWorld{wd:<ME::GE as GlobalEvent>::WD},
    SendMeEverything
}
#[derive(Clone)]
pub struct HordeServerData<ME:MultiplayerEngine> {
    time_travel:TimeTravelData<ME>,
    tickrate:usize,
    player_id_generator:Arc<AtomicUsize>,
    tcp:Arc<RwLock<HordeTcpServerData<ME>>>,
    local_tcp_buffer:Vec<u8>,
    local_bytes_to_decode:Vec<u8>,
    local_decode_buffer:Vec<u8>,
    local_decoder:<HordeMultiplayerPacket<ME> as FromBytes>::Decoder,
    events_to_spread:Receiver<ME::GE>,
    must_apply:(Sender<ME::GE>, Receiver<ME::GE>),
    must_set:(Sender<(ME::ID, <ME::GE as GlobalEvent>::GC)>, Receiver<(ME::ID, <ME::GE as GlobalEvent>::GC)>)

}
#[derive(Clone)]
pub struct HordeTcpServerData<ME:MultiplayerEngine> {
    listener:Arc<RwLock<TcpListener>>,
    streams:HashMap<usize, (Arc<RwLock<TcpStream>>, SocketAddr, (Sender<HordeMultiplayerPacket<ME>>, Receiver<HordeMultiplayerPacket<ME>>))>,
    connected_players:Vec<usize>,
    connected_counter:ParallelCounter,
}

impl<ME:MultiplayerEngine> HordeTcpServerData<ME> {
    fn new<A:ToSocketAddrs>(adress:A) -> Self {
        let mut listener = TcpListener::bind(adress).expect("Tcp binding error : ");
        listener.set_nonblocking(true);
        Self { listener: Arc::new(RwLock::new(listener)), streams: HashMap::with_capacity(64), connected_players:Vec::with_capacity(64), connected_counter:ParallelCounter::new(0, 1) }
    }
}

impl<ME:MultiplayerEngine> HordeServerData<ME> {
    fn new<A:ToSocketAddrs>(tick_tolerance:usize, adress:A, events_to_spread:Receiver<ME::GE>, tickrate:usize) -> Self {
        Self { 
            time_travel: TimeTravelData { history: Vec::with_capacity(tick_tolerance), latest_tick:0 },
            tickrate,
            tcp:Arc::new(RwLock::new(HordeTcpServerData::new(adress))),
            player_id_generator:Arc::new(AtomicUsize::new(0)),
            local_tcp_buffer:vec![0; 1024],
            local_decode_buffer:Vec::with_capacity(1024),
            local_bytes_to_decode:Vec::with_capacity(1024),
            local_decoder:<HordeMultiplayerPacket<ME> as FromBytes>::get_decoder(),
            events_to_spread,
            must_apply:unbounded(),
            must_set:unbounded()
        }
    }
    fn try_handshake(&mut self) -> Option<(String, usize)> {
        let mut tcp_write = self.tcp.write().unwrap();
        tcp_write.connected_counter.reset();
        let (result, extras) = match tcp_write.listener.write().unwrap().accept() {
            Ok((mut new_stream, adress)) => {
                println!("NEW STREAM");
                new_stream.set_nonblocking(true);
                let mut decoder = HordeMultiplayerPacket::<ME>::get_decoder();
                let mut tcp_buffer = Vec::with_capacity(1024);
                let mut got_first_response = false;
                let mut decoded_pseudonym = String::new();
                let mut given_id = 0;
                while !got_first_response {
                    let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut self.local_decoder, &mut new_stream, &mut self.local_tcp_buffer, &mut self.local_decode_buffer);
                    for event in events {
                        match event {
                            HordeMultiplayerPacket::WannaJoin(player_name) => {decoded_pseudonym = player_name.clone(); got_first_response = true;}
                            _ => panic!("Player didn't send right connection packet"),
                        }
                    }
                    
                }
                
                given_id = self.player_id_generator.fetch_add(1, Ordering::Relaxed);
                println!("HANDSHOOK {} | GAVE ID {}", decoded_pseudonym, given_id);
                tcp_buffer.clear();
                HordeMultiplayerPacket::<ME>::ThatsUrPlayerID(given_id, self.tickrate).add_bytes(&mut tcp_buffer);
                new_stream.write(&tcp_buffer).unwrap();
                
                (Some((decoded_pseudonym, given_id)), Some((given_id, new_stream, adress, tcp_write.streams.len())))
            },
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (None, None),
            Err(bad_error) => panic!("Bad TCP listening error : {}", bad_error), 
        };
        match extras {
            Some((given_id, new_stream, adress, cool_len)) => {
                tcp_write.streams.insert(given_id, (Arc::new(RwLock::new(new_stream)), adress, unbounded()));
                tcp_write.connected_counter.update_len(tcp_write.streams.len());
            }
            None => ()
        }

        result
    }
    fn decode_events(&mut self, stream:&mut TcpStream) -> Vec<HordeMultiplayerPacket<ME>> {
        let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut self.local_decoder, stream, &mut self.local_tcp_buffer, &mut self.local_decode_buffer);
        events
    }
    fn get_response_to_event(&mut self, event:HordeMultiplayerPacket<ME>, engine:&ME) -> Vec<HordeMPServerResponse<ME>> {
        let mut responses = Vec::with_capacity(32);

        match &event {
            HordeMultiplayerPacket::Chat { from_player, text } => responses.push(HordeMPServerResponse::ToEveryone(event.clone())),
            HordeMultiplayerPacket::DoYouAgree(global_event) => {
                let event_tick = engine.get_tick(global_event);
                let calculated_event_tick = event_tick.abs_diff(self.time_travel.latest_tick);
                match engine.get_event_origin(global_event) {
                    Some(event_origin_id) => {
                        if calculated_event_tick < self.time_travel.history.len() {
                            match self.time_travel.history[calculated_event_tick].events.get(&event_origin_id) {
                                Some(events) => if !events.contains(global_event) {
                                    let affected = self.time_travel.get_affected_from(&event_origin_id, calculated_event_tick);
                                    for aff in affected {
                                        let components = engine.get_components_to_sync_for(&aff);
                                        for compo in components {
                                            responses.push(HordeMPServerResponse::BackToSender(HordeMultiplayerPacket::ResetComponent { id: aff.clone(), data: compo }))
                                        }
                                    }
                                },
                                None => ()
                            }
                        }
                    },
                    None => ()
                }
            },
            HordeMultiplayerPacket::DoYouAgreeComponent(id, component) => {
                if !engine.is_that_component_correct(id, component) {
                    let event_tick = self.time_travel.latest_tick - self.time_travel.history.len();
                    let event_origin_id = id;
                    let calculated_event_tick = event_tick.abs_diff(self.time_travel.latest_tick);
                    if calculated_event_tick < self.time_travel.history.len() {
                        match self.time_travel.history[calculated_event_tick].events.get(&event_origin_id) {
                            Some(events) => {
                                let affected = self.time_travel.get_affected_from(&event_origin_id, calculated_event_tick);
                                for aff in affected {
                                    let components = engine.get_components_to_sync_for(&aff);
                                    for compo in components {
                                        responses.push(HordeMPServerResponse::BackToSender(HordeMultiplayerPacket::ResetComponent { id: aff.clone(), data: compo }))
                                    }
                                }
                            },
                            None => ()
                        }
                    }
                }
            },
            HordeMultiplayerPacket::PlayerJoined(player) => panic!("Server Received PlayerJoined"),
            HordeMultiplayerPacket::ResetComponent { id, data } => {responses.push(HordeMPServerResponse::ToEveryoneElse(event.clone())); self.must_set.0.send((id.clone(), data.clone())); },
            HordeMultiplayerPacket::ResetWorld { wd } => panic!("Server was told to reset world"),
            HordeMultiplayerPacket::SpreadEvent(global_evt) => {responses.push(HordeMPServerResponse::ToEveryoneElse(event.clone())); self.must_apply.0.send(global_evt.clone()); },//engine.apply_event(global_evt.clone())},
            HordeMultiplayerPacket::WannaJoin(_) => panic!("Player that has already joined asked again"),
            HordeMultiplayerPacket::ThatsUrPlayerID(_, _) => panic!("Player tried to tell server a player ID"),
            HordeMultiplayerPacket::SendMeEverything => {
                let (components, world) = engine.get_all_components_and_world();
                for (id, component) in components {
                    responses.push(HordeMPServerResponse::BackToSender(HordeMultiplayerPacket::ResetComponent { id, data: component }));
                }
                responses.push(HordeMPServerResponse::BackToSender(HordeMultiplayerPacket::ResetWorld { wd: world }));
            },
        }

        responses
    }

    /// Sequential, first in line
    fn handshakes_players_events(&mut self, engine:&mut ME, players:&mut HordePlayers<ME::ID>) {
        let new_players = self.listen_for_new_handshakes();
        {
            let mut tcp_write = self.tcp.write().unwrap();
            for new_player in new_players {
                tcp_write.connected_players.push(new_player.1);
                players.players.write().unwrap().push(HordePlayer {ent_id:None, player_name:new_player.0.clone(), player_id:new_player.1});
                for (player, (_, _, (sender, _))) in tcp_write.streams.iter() {
                    sender.send(HordeMultiplayerPacket::PlayerJoined(HordePlayer {ent_id:None, player_name:new_player.0.clone(), player_id:new_player.1})).unwrap();
                }
            }
        }

        while let Ok(event) = self.must_apply.1.try_recv() {
            engine.apply_event(event);
        }
        while let Ok((id, comp)) = self.must_set.1.try_recv() {
            engine.set_component(id, comp);
        }
        self.reset_counters();
    }

    /// Sequential
    fn listen_for_new_handshakes(&mut self) -> Vec<(String, usize)> {
        let mut new_players = Vec::with_capacity(4);
        while let Some((pseudonym, id)) = self.try_handshake() {
            new_players.push((pseudonym, id));
        }
        new_players
    }
    ///  Multithreadable but made to be Sequential
    pub fn share_must_spread(&mut self) {
        let mut tcp = self.tcp.read().unwrap();
        while let Ok(global_event) = self.events_to_spread.try_recv() {
            for player in &tcp.connected_players {
                match tcp.streams.get(player) {
                    Some((_,_,(sender, recv))) => sender.send(HordeMultiplayerPacket::SpreadEvent(global_event.clone())).unwrap(),
                    None => panic!("Player wasn't removed from players list"),
                }
            }
        }
    }
    /// Multithreadable, after hanshakes_players_events
    fn streams_share_spread(&mut self, engine:&ME) {
        self.iterate_over_streams(engine);
        self.share_must_spread();
    }
    /// Multithreadable
    pub fn iterate_over_streams(&mut self, engine:&ME) {
        let mut counter = {
            let mut tcp_read = self.tcp.read().unwrap();
            let mut counter = tcp_read.connected_counter.clone();
            counter.initialise();
            counter
        };
        for i in counter {
            let (stream, addr, _) = {
                let mut tcp_read = self.tcp.read().unwrap();
                let player = tcp_read.connected_players[i];
                match tcp_read.streams.get(&player) {
                    Some((stream, addr, pair)) => {
                        //let mut stream_write = stream.write().unwrap();
                        (stream.clone(), addr.clone(), pair.clone())
                    },
                    None => panic!("Big problem, player {} not found", player)
                }
            };
            let mut stream_write = stream.write().unwrap();
            let events = self.decode_events(&mut stream_write);
            for event in events {
                let responses = self.get_response_to_event(event, engine);
                for response in responses {
                    match response {
                        HordeMPServerResponse::BackToSender(rep) => {
                            self.send_packet_to(&mut stream_write, rep);
                        },
                        HordeMPServerResponse::ToEveryone(rep) => {
                            let tcp_read = self.tcp.read().unwrap();
                            for (stream, addr, (sender, receiver)) in tcp_read.streams.values() {
                                sender.send(rep.clone()).unwrap();
                            }
                        },
                        HordeMPServerResponse::ToEveryoneElse(rep) => {
                            let tcp_read = self.tcp.read().unwrap();
                            for (stream, target_addr, (sender, receiver)) in tcp_read.streams.values() {
                                if *target_addr != addr {
                                    sender.send(rep.clone()).unwrap();
                                }
                                
                            }
                        },
                    }
                }
            }
        }
    }

    /// Multithreadable
    fn send_packet_to(&self, stream:&mut TcpStream, packet:HordeMultiplayerPacket<ME>) {
        let mut bytes = Vec::with_capacity(packet.get_bytes_size());
        packet.add_bytes(&mut bytes);
        stream.write_all(&bytes);
    }

    /// Sequential, called on its own after streams_share_spread
    fn reset_counters(&self) {
        let mut tcp_write = self.tcp.write().unwrap();
        tcp_write.connected_counter.reset();
    }

    /// Multithreadable, called after rest_counters is called on its own
    fn send_held_up_packets(&mut self) {
        let mut counter = {
            let mut tcp_read = self.tcp.read().unwrap();
            let mut counter = tcp_read.connected_counter.clone();
            counter.initialise();
            counter
        };
        for i in counter {
            let (stream, addr, (_, recv)) = {
                let mut tcp_read = self.tcp.read().unwrap();
                let player = tcp_read.connected_players[i];
                match tcp_read.streams.get(&player) {
                    Some((stream, addr, pair)) => {
                        //let mut stream_write = stream.write().unwrap();
                        (stream.clone(), addr.clone(), pair.clone())
                    },
                    None => panic!("Big problem, player {} not found", player)
                }
            };
            let mut stream_write = stream.write().unwrap();
            while let Ok(packet) = recv.try_recv() {
                self.send_packet_to(&mut stream_write, packet);
            }
        }
    }
}
#[derive(Clone)]
pub enum HordeMPServerResponse<ME:MultiplayerEngine> {
    BackToSender(HordeMultiplayerPacket<ME>),
    ToEveryone(HordeMultiplayerPacket<ME>),
    ToEveryoneElse(HordeMultiplayerPacket<ME>),
}
#[derive(Clone)]
pub enum HordeMultiplayerMode<ME:MultiplayerEngine> {
    Server(HordeServerData<ME>),
    Client(HordeClientData<ME>)
}

#[derive(Clone)]
pub struct HordeMultiplayer<ME:MultiplayerEngine> {
    mode:HordeMultiplayerMode<ME>,
    players:HordePlayers<ME::ID>,
}

impl<ME:MultiplayerEngine> HordeMultiplayer<ME> {
    /// Call first as server
    pub fn handshakes_players_events(&mut self, engine:&mut ME) {
        // println!("SERVER : HANDSHAKES");
        match &mut self.mode {
            HordeMultiplayerMode::Client(_) => panic!("Not a server"),
            HordeMultiplayerMode::Server(server_data) => server_data.handshakes_players_events(engine, &mut self.players),
        }
    }
    /// Call second as server, may be multihreadable
    pub fn streams_share_spread(&mut self, engine:&ME) {
        // println!("SERVER : SHARESPREAD");
        match &mut self.mode {
            HordeMultiplayerMode::Client(_) => panic!("Not a server"),
            HordeMultiplayerMode::Server(server_data) => server_data.streams_share_spread(engine),
        }
    }
    /// Call third as server
    pub fn reset_server_counters(&mut self) {
        // println!("SERVER : RESET");
        match &mut self.mode {
            HordeMultiplayerMode::Client(_) => panic!("Not a server"),
            HordeMultiplayerMode::Server(server_data) => server_data.reset_counters(),
        }
    }
    /// Call fourth as server, multithreaded
    pub fn send_held_up_packets(&mut self) {
        // println!("SERVER : SENDING HELD UP PACKETS");
        match &mut self.mode {
            HordeMultiplayerMode::Client(_) => panic!("Not a server"),
            HordeMultiplayerMode::Server(server_data) => server_data.send_held_up_packets(),
        }
    }
    /// Call first as client
    pub fn receive_all_events_and_respond(&mut self, engine:&mut ME) {
        match &mut self.mode {
            HordeMultiplayerMode::Client(client) => client.tcp.as_mut().unwrap().write().unwrap().receive_all_events_and_respond(&client.events_to_spread, &mut self.players, engine, &client.chat, client.id.unwrap(), client.client_ent_ids.read().unwrap().clone()),
            HordeMultiplayerMode::Server(_) => panic!("Not a client"),
        }
    }
    /// Call second as client
    pub fn send_all_events(&mut self) {
        match &mut self.mode {
            HordeMultiplayerMode::Client(client) => client.tcp.as_mut().unwrap().write().unwrap().send_all_events(&client.events_to_spread, &client.chat, client.id.unwrap(), ),
            HordeMultiplayerMode::Server(_) => panic!("Not a client"),
        }
    } 
}

#[derive(Clone)]
pub enum HordeMultiModeChoice {
    Server{adress:(Ipv4Addr, u16), max_players:usize, tick_tolerance:usize, tickrate:usize},
    Client{adress:Option<(Ipv4Addr, u16)>, name:String, chat:Receiver<String>},
}

impl<ME:MultiplayerEngine> HordeMultiplayer<ME> {
    pub fn new(mode:HordeMultiModeChoice, events_to_spread:Receiver<ME::GE>) -> Self {
        match mode {
            HordeMultiModeChoice::Client { adress, name, chat } => {
                let final_mode = HordeMultiplayerMode::Client(HordeClientData::new(name, Some((adress.unwrap().0, adress.unwrap().1)), events_to_spread, chat));
                Self { mode:final_mode, players:HordePlayers::new(3) }
            },
            HordeMultiModeChoice::Server { adress, max_players, tick_tolerance, tickrate } => {

                let final_mode = HordeMultiplayerMode::Server(HordeServerData::new(tick_tolerance, (adress.0, adress.1), events_to_spread, tickrate));
                Self { mode:final_mode, players:HordePlayers::new(max_players) }
            },
            
        }
        
    }
}

#[derive(Clone, ToBytes, FromBytes, Debug)]
pub struct HordePlayer<ID:Identify> {
    ent_id:Option<ID>,
    player_name:String,
    player_id:usize,
}

#[derive(Clone)]
pub struct HordePlayers<ID:Identify> {
    players:Arc<RwLock<Vec<HordePlayer<ID>>>>,
    max_players:Arc<AtomicUsize>
}

impl<ID:Identify> HordePlayers<ID> {
    pub fn new(expected_players:usize) -> Self {
        Self { players: Arc::new(RwLock::new(Vec::with_capacity(expected_players))), max_players: Arc::new(AtomicUsize::new(expected_players)) }
    }
}
#[derive(Clone)]
pub struct HordeClientData<ME:MultiplayerEngine> {
    name:String,
    id:Option<usize>,
    tickrate:Option<usize>,
    tcp:Option<Arc<RwLock<HordeClientTcp<ME>>>>,
    events_to_spread:Receiver<ME::GE>,
    chat:Receiver<String>,
    client_ent_ids:Arc<RwLock<Vec<ME::ID>>>
}

impl<ME:MultiplayerEngine> HordeClientData<ME> {
    pub fn new(name:String, adress:Option<(Ipv4Addr, u16)>, events_to_spread:Receiver<ME::GE>, chat:Receiver<String>) -> Self {
       
        let mut self_cool = Self {
            name:name.clone(),
            tickrate:None,
            id: None,
            tcp: None,
            events_to_spread,
            chat,
            client_ent_ids:Arc::new(RwLock::new(Vec::with_capacity(6)))
        };
        match adress {
            Some(addr) => {
                let (tcp, id, tickrate) = HordeClientTcp::new(addr, name);
                self_cool.id = Some(id);
                self_cool.tickrate = Some(tickrate);
                self_cool.tcp = Some(Arc::new(RwLock::new(tcp)));
            },
            None => ()
        }
        self_cool
    }
    pub fn add_client_ent_id(&self, id:ME::ID) {
        self.client_ent_ids.write().unwrap().push(id);
    }
    pub fn remove_client_ent_id(&self, id:ME::ID) {
        let mut ids = self.client_ent_ids.write().unwrap();
        if let Some((i,candidate)) = ids.iter().enumerate().find(|(i, candidate)| {**candidate == id}) {
            ids.remove(i);
        }
    }
    pub fn get_tickrate(&self) -> Option<usize> {
        self.tickrate
    }
}
pub struct HordeClientTcp<ME:MultiplayerEngine> {
    stream:TcpStream,
    adress:(Ipv4Addr, u16),
    local_tcp_buffer:Vec<u8>,
    local_decode_buffer:Vec<u8>,
    local_decoder:<HordeMultiplayerPacket<ME> as FromBytes>::Decoder,
    id_generator:ME::RIDG,
}

impl<ME:MultiplayerEngine> HordeClientTcp<ME> {
    fn new(adress:(Ipv4Addr, u16), name:String) -> (Self, usize, usize) {
        let mut stream = TcpStream::connect_timeout(&adress.into(), Duration::from_secs(3)).unwrap();
        stream.set_nonblocking(true);
        let mut buffer = Vec::with_capacity(1024);
        HordeMultiplayerPacket::<ME>::WannaJoin(name).add_bytes(&mut buffer);
        stream.write(&buffer).expect("Got an error while sending for handshake");
        buffer.clear();
        let mut id = 0;
        let mut tickrate = 0;
        let mut given_id = false;
        let mut decoder:HordeMultiplayerPacketDecoder<ME> = HordeMultiplayerPacket::get_decoder();
        let mut local_tcp_buffer = vec![0 ; 1024];
        let mut local_decode_buffer = Vec::with_capacity(1024);
        let mut local_decoder = HordeMultiplayerPacket::<ME>::get_decoder();
        while !given_id {
            let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut local_decoder, &mut stream, &mut local_tcp_buffer, &mut local_decode_buffer);
            for event in events {
                match event {
                    HordeMultiplayerPacket::ThatsUrPlayerID(new_id, tick) => {
                        id = new_id;
                        given_id = true;
                        tickrate = tick;
                    },
                    _ => panic!("Server didn't do the right handshake")
                }
            }
        }
        buffer.clear();
        HordeMultiplayerPacket::<ME>::SendMeEverything.add_bytes(&mut buffer);
        stream.write(&mut buffer);
        (Self {id_generator:ME::get_random_id_generator(),stream, adress, local_tcp_buffer:vec![0 ; 1024], local_decode_buffer:Vec::with_capacity(1024), local_decoder:HordeMultiplayerPacket::<ME>::get_decoder()}, id, tickrate)
    }
    fn decode_events(&mut self) -> Vec<HordeMultiplayerPacket<ME>> {
        let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut self.local_decoder, &mut self.stream, &mut self.local_tcp_buffer, &mut self.local_decode_buffer);
        events
    }
    fn send_packet(&mut self, packet:HordeMultiplayerPacket<ME>) {
        let mut bytes = Vec::with_capacity(packet.get_bytes_size());
        packet.add_bytes(&mut bytes);
        self.stream.write_all(&bytes).unwrap();
    }
    fn get_response_to(&mut self, packet:HordeMultiplayerPacket<ME>, players:&mut HordePlayers<ME::ID>, engine:&mut ME) -> Vec<HordeMultiplayerPacket<ME>> {
        let mut response = Vec::with_capacity(4);
        match packet {
            HordeMultiplayerPacket::Chat { from_player, text } => println!("{} : {}", from_player, text),
            HordeMultiplayerPacket::DoYouAgree(_) => panic!("Server sent agree packet, impossible"),
            HordeMultiplayerPacket::PlayerJoined(player) => players.players.write().unwrap().push(player),
            HordeMultiplayerPacket::ResetComponent { id, data } => {
                engine.set_component(id, data);
            },
            HordeMultiplayerPacket::ResetWorld { wd } => {
                engine.set_world(wd);
            },
            HordeMultiplayerPacket::SpreadEvent(evt) => {
                engine.apply_event(evt);
            },
            HordeMultiplayerPacket::SendMeEverything => panic!("Server cannot ask to be sent everything"),
            HordeMultiplayerPacket::ThatsUrPlayerID(_, _) => panic!("Server can't send player ID past handshake"),
            HordeMultiplayerPacket::WannaJoin(_) => panic!("Serve can't want to join your server"),
            HordeMultiplayerPacket::DoYouAgreeComponent(_, _) => panic!("Server can't ask for component"),

        }
        response
    }
    /// Call last
    fn send_all_events(&mut self, events_to_spread:&Receiver<ME::GE>, chat:&Receiver<String>, id:usize) {
        while let Ok(global_event) = events_to_spread.try_recv() {
            self.send_packet(HordeMultiplayerPacket::SpreadEvent(global_event));
        }
        while let Ok(chat_line) = chat.try_recv() {
            self.send_packet(HordeMultiplayerPacket::Chat { from_player:id, text:chat_line });
        }
        
    }
    /// Call first
    fn receive_all_events_and_respond(&mut self, events_to_spread:&Receiver<ME::GE>, players:&mut HordePlayers<ME::ID>, engine:&mut ME, chat:&Receiver<String>, id:usize, client_ids:Vec<ME::ID>) {
        self.send_all_events(events_to_spread, chat, id);
        for id in client_ids {
            for compo in engine.get_components_to_sync_for(&id) {
                self.send_packet(HordeMultiplayerPacket::ResetComponent { id:id.clone(), data: compo });
            }
        }
        let packets = self.decode_events();
        for packet in packets {
            let response = self.get_response_to(packet, players, engine);
            for resp in response {
                self.send_packet(resp);
            }
        }
        for i in 0..10 {
            let random = engine.generate_random_id(&mut self.id_generator);
            match random {
                Some(random) => {
                    let stuff = engine.get_components_to_sync_for(&random);
                    for component in stuff {
                        self.send_packet(HordeMultiplayerPacket::DoYouAgreeComponent(random.clone(), component));
                    }
                },
                None => ()
            }
            
        }
    }
}

#[derive(Clone)]
pub struct TimeTravelData<ME:MultiplayerEngine> {
    history:Vec<HordeEventReport<ME::ID, ME::GE>>,
    latest_tick:usize,
}

impl<ME:MultiplayerEngine> TimeTravelData<ME> {
    fn get_affected_from(&self, id:&ME::ID, tick_back:usize) -> Vec<ME::ID> {
        let mut total_affected = Vec::with_capacity(16);
        total_affected.push(id.clone());
        let mut affected_last_tick = Vec::with_capacity(16);
        affected_last_tick.push(id.clone());
        let mut searching_tick = tick_back;
        let mut total_affected_set = HashSet::with_capacity(32);
        if self.history.len() > searching_tick {
            loop {
                let mut affected_this_tick = Vec::with_capacity(16);
                let mut affected_this_tick_set = HashSet::with_capacity(16);
                for affected in &affected_last_tick {
                    match self.history[searching_tick].events.get(affected) {
                        Some(events) => for event in events {
                            let id = ME::get_target(event);
                            if !total_affected_set.contains(&id) {
                                total_affected.push(id.clone());
                                total_affected_set.insert(id.clone());
                            }
                            if !affected_this_tick_set.contains(&id) {
                                affected_this_tick.push(id.clone());
                                affected_this_tick_set.insert(id.clone());
                            }
                        },
                        None => ()
                    }
                }
                if searching_tick == 0 {
                    break;
                }
                searching_tick -= 1;
                affected_last_tick = affected_this_tick;
            }
        }
        
        total_affected
    }
}
#[derive(Clone)]
pub struct HordeEventReport<ID:Identify, GE:GlobalEvent> {
    events:HashMap<ID, Vec<GE>>,
    tick:usize,
}

impl<ID:Identify, GE:GlobalEvent> HordeEventReport<ID, GE> {
    pub fn new(the_map:HashMap<ID, Vec<GE>>, tick:usize) -> Self {
        Self {
            events:the_map,
            tick
        }
    }
}

