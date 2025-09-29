use std::{collections::{HashMap, HashSet}, hash::Hash, io::{self, ErrorKind, Read, Write}, net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs}, ops::Mul, sync::{atomic::{AtomicUsize, Ordering}, mpmc::{channel, Receiver, Sender}, Arc, RwLock}, thread, time::Duration};

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
    fn get_total_len(&self) -> usize;
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
    events_to_spread:Receiver<ME::GE>,
    must_apply:(Sender<ME::GE>, Receiver<ME::GE>),
    must_set:(Sender<(ME::ID, <ME::GE as GlobalEvent>::GC)>, Receiver<(ME::ID, <ME::GE as GlobalEvent>::GC)>)

}

struct TcpStreamHandler<ME:MultiplayerEngine> {
    stream:TcpStream,
    local_tcp_buffer:Vec<u8>,
    local_decode_buffer:Vec<u8>,
    local_decoder:<HordeMultiplayerPacket<ME> as FromBytes>::Decoder,
    events_to_send:Receiver<Vec<u8>>,
    decoded_events:Sender<HordeMultiplayerPacket<ME>>,
    tickrate_duration:Duration
}

impl<ME:MultiplayerEngine + 'static> TcpStreamHandler<ME> {
    pub fn initiate(stream:TcpStream,local_tcp_buffer:Vec<u8>,local_decode_buffer:Vec<u8>,local_decoder:<HordeMultiplayerPacket<ME> as FromBytes>::Decoder, tickrate:usize) -> (Sender<Vec<u8>>, Receiver<HordeMultiplayerPacket<ME>>) {
        let (sender, events_to_send) = channel();
        let (decoded_events, receiver) = channel();

        thread::spawn(move || {
            TcpStreamHandler {stream, local_decode_buffer, local_decoder, local_tcp_buffer, events_to_send, decoded_events, tickrate_duration:Duration::from_secs_f64(1.0)}.handling_loop();
        });

        (sender, receiver)
    }
    pub fn handling_loop(&mut self) {
        loop {
            self.read_from_stream();
            self.write_to_stream();
        }
    }
    pub fn read_from_stream(&mut self) {
        // println!("[TCP Handler] Reading from stream with {} bytes in my decoder", self.local_decode_buffer.len()); 
        let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut self.local_decoder, &mut self.stream, &mut self.local_tcp_buffer, &mut self.local_decode_buffer);
        // println!("[TCP Handler] Finished reading from stream with {} bytes in my decoder and {} events decoded", self.local_decode_buffer.len(), events.len()); 
        for event in events {
            self.decoded_events.send(event).unwrap();
        }
    }
    pub fn write_to_stream(&mut self) {
        while let Ok(data) = self.events_to_send.try_recv() {
            let mut start = 0;
            loop {
                // println!("Inside writing loop with start = {} and len = {} and queue size = {}", start, data.len(), self.events_to_send.len());
                match self.stream.write(&data[start..]) {
                    Ok(bytes_written) => {
                        start += bytes_written;
                        self.stream.flush().unwrap();
                        if start == data.len() {
                            break;
                        }
                    },
                    Err(error) => if error.kind() == ErrorKind::WouldBlock {
                        continue;
                    }
                    else {
                        panic!("writing error {}", error);
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct HordeTcpServerData<ME:MultiplayerEngine> {
    listener:Arc<RwLock<TcpListener>>,
    streams:HashMap<usize, (SocketAddr, (Sender<Vec<u8>>, Receiver<HordeMultiplayerPacket<ME>>), Arc<RwLock<ME::RIDG>>)>,
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

impl<ME:MultiplayerEngine + 'static> HordeServerData<ME> {
    fn new<A:ToSocketAddrs>(tick_tolerance:usize, adress:A, events_to_spread:Receiver<ME::GE>, tickrate:usize) -> Self {
        Self { 
            time_travel: TimeTravelData { history: Vec::with_capacity(tick_tolerance), latest_tick:0 },
            tickrate,
            tcp:Arc::new(RwLock::new(HordeTcpServerData::new(adress))),
            player_id_generator:Arc::new(AtomicUsize::new(0)),
            events_to_spread,
            must_apply:channel(),
            must_set:channel()
        }
    }
    fn try_handshake(&mut self) -> Option<(String, usize)> {

        //println!("[Multiplayer server] Starting a handshake");
        let mut tcp_write = self.tcp.write().unwrap();
        tcp_write.connected_counter.reset();
        let (result, extras) = match tcp_write.listener.write().unwrap().accept() {
            Ok((mut new_stream, adress)) => {
                //println!("NEW STREAM");
                new_stream.set_read_timeout(Some(Duration::from_secs_f64((1.0/(self.tickrate as f64)) * 0.5)));
                let mut decoder = HordeMultiplayerPacket::<ME>::get_decoder();
                let mut got_first_response = false;
                let mut decoded_pseudonym = String::new();
                let mut given_id = 0;
                let mut decoder = <HordeMultiplayerPacket<ME> as FromBytes>::get_decoder();
                let mut tcp_buffer = vec![0 ; 4096];
                let mut decode_buffer = Vec::with_capacity(1024);
                while !got_first_response {
                    println!("[Multiplayer server] Reading events from client during handshake");
                    let events = decode_from_tcp::<false, HordeMultiplayerPacket<ME>>(&mut decoder, &mut new_stream, &mut tcp_buffer, &mut decode_buffer);
                    for event in events {
                        match event {
                            HordeMultiplayerPacket::WannaJoin(player_name) => {decoded_pseudonym = player_name.clone(); got_first_response = true;}
                            _ => panic!("Player didn't send right connection packet"),
                        }
                    }
                    
                }

                given_id = self.player_id_generator.fetch_add(1, Ordering::Relaxed);
                //println!("HANDSHOOK {} | GAVE ID {}", decoded_pseudonym, given_id);
                tcp_buffer.clear();
                HordeMultiplayerPacket::<ME>::ThatsUrPlayerID(given_id, self.tickrate).add_bytes(&mut tcp_buffer);
                new_stream.write(&tcp_buffer).unwrap();
                println!("[Multiplayer server] Sent player ID");
                (Some((decoded_pseudonym, given_id)), Some((given_id, new_stream, adress, tcp_write.streams.len(), decoder, tcp_buffer, decode_buffer)))
            },
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (None, None),
            Err(bad_error) => panic!("Bad TCP listening error : {}", bad_error), 
        };
        match extras {
            Some((given_id, new_stream, adress, cool_len, decoder, tcp_buffer, decode_buffer)) => {
                tcp_write.streams.insert(given_id, (adress, TcpStreamHandler::initiate(new_stream, tcp_buffer, decode_buffer, decoder, self.tickrate), Arc::new(RwLock::new(ME::get_random_id_generator()))));
                tcp_write.connected_counter.update_len(tcp_write.streams.len());
            }
            None => ()
        }

        result
    }
    fn get_response_to_event(&mut self, event:HordeMultiplayerPacket<ME>, engine:&ME) -> Vec<HordeMPServerResponse<ME>> {
        let mut responses = Vec::with_capacity(32);

        match &event {
            HordeMultiplayerPacket::Chat { from_player, text } => responses.push(HordeMPServerResponse::ToEveryone(event.clone())),
            HordeMultiplayerPacket::DoYouAgree(global_event) => {
                //println!("[Multiplayer server] got DoYouAgree");
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

                //println!("[Multiplayer server] got DoYouAgreeComponent");
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
            HordeMultiplayerPacket::ResetComponent { id, data } => {

                //println!("[Multiplayer server] got ResetComponent");
                responses.push(HordeMPServerResponse::ToEveryoneElse(event.clone()));
                self.must_set.0.send((id.clone(), data.clone()));
            },
            HordeMultiplayerPacket::ResetWorld { wd } => panic!("Server was told to reset world"),
            HordeMultiplayerPacket::SpreadEvent(global_evt) => {
                //println!("[Multiplayer server] got SpreadEvent");
                responses.push(HordeMPServerResponse::ToEveryoneElse(event.clone()));
                self.must_apply.0.send(global_evt.clone());

            },//engine.apply_event(global_evt.clone())},
            HordeMultiplayerPacket::WannaJoin(_) => panic!("Player that has already joined asked again"),
            HordeMultiplayerPacket::ThatsUrPlayerID(_, _) => panic!("Player tried to tell server a player ID"),
            HordeMultiplayerPacket::SendMeEverything => {
                //println!("[Multiplayer server] Sending Everything");
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
                for (player, (_, (sender, _), _)) in tcp_write.streams.iter() {
                    sender.send(HordeMultiplayerPacket::<ME>::PlayerJoined(HordePlayer {ent_id:None, player_name:new_player.0.clone(), player_id:new_player.1}).get_bytes_vec()).unwrap();
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
        //println!("[Multiplayer server] Share must spread");
        let mut tcp = self.tcp.read().unwrap();
        while let Ok(global_event) = self.events_to_spread.try_recv() {
            for player in &tcp.connected_players {
                match tcp.streams.get(player) {
                    Some((_, (sender, recv), _)) => sender.send(HordeMultiplayerPacket::<ME>::SpreadEvent(global_event.clone()).get_bytes_vec()).unwrap(),
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
            let (addr, (sender, recv), random) = {
                let mut tcp_read = self.tcp.read().unwrap();
                let player = tcp_read.connected_players[i];
                match tcp_read.streams.get(&player) {
                    Some((addr, pair, random_gen)) => {
                        //let mut stream_write = stream.write().unwrap();
                        (addr.clone(), pair.clone(), random_gen.clone())
                    },
                    None => panic!("Big problem, player {} not found", player)
                }
            };
            while let Ok(event) = recv.try_recv() {
                let responses = self.get_response_to_event(event, engine);
                for response in responses {
                    match response {
                        HordeMPServerResponse::BackToSender(rep) => {
                            sender.send(rep.get_bytes_vec()).unwrap();
                        },
                        HordeMPServerResponse::ToEveryone(rep) => {
                            let tcp_read = self.tcp.read().unwrap();
                            let bytes = rep.get_bytes_vec();
                            for (addr, (sender, receiver), _) in tcp_read.streams.values() {
                                sender.send(bytes.clone()).unwrap();
                            }
                        },
                        HordeMPServerResponse::ToEveryoneElse(rep) => {
                            let bytes = rep.get_bytes_vec();
                            let tcp_read = self.tcp.read().unwrap();
                            for (target_addr, (sender, receiver), _) in tcp_read.streams.values() {
                                if *target_addr != addr {
                                    sender.send(bytes.clone()).unwrap();
                                }
                                
                            }
                        },
                    }
                }
            }
            let mut gen_mut = random.write().unwrap();
            let number_of_randoms = engine.get_total_len()/self.tickrate + 1;
            for i in 0..number_of_randoms {
                let random = engine.generate_random_id(&mut gen_mut);
                match random {
                    Some(random) => {
                        let stuff = engine.get_components_to_sync_for(&random);
                        for component in stuff {
                            sender.send(HordeMultiplayerPacket::<ME>::ResetComponent { id: random.clone(), data: component }.get_bytes_vec()).unwrap();
                        }
                    },
                    None => ()
                }
            }
        }
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
            let (addr, (sender, recv)) = {
                let mut tcp_read = self.tcp.read().unwrap();
                let player = tcp_read.connected_players[i];
                match tcp_read.streams.get(&player) {
                    Some((addr, pair, _)) => {
                        //let mut stream_write = stream.write().unwrap();
                        (addr.clone(), pair.clone())
                    },
                    None => panic!("Big problem, player {} not found", player)
                }
            };
            while let Ok(packet) = recv.try_recv() {
                sender.send(packet.get_bytes_vec());
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
pub enum HordeMultiplayerMode<ME:MultiplayerEngine + 'static> {
    Server(HordeServerData<ME>),
    Client(HordeClientData<ME>)
}

#[derive(Clone)]
pub struct HordeMultiplayer<ME:MultiplayerEngine + 'static> {
    mode:HordeMultiplayerMode<ME>,
    players:HordePlayers<ME::ID>,
}

impl<ME:MultiplayerEngine + 'static> HordeMultiplayer<ME> {
    pub fn get_tickrate(&self) -> Option<usize> {
        match &self.mode {
            HordeMultiplayerMode::Server(server_data) => Some(server_data.tickrate),
            HordeMultiplayerMode::Client(client_data) => client_data.tickrate
        }
    }
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

impl<ME:MultiplayerEngine + 'static> HordeMultiplayer<ME> {
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
pub struct HordeClientData<ME:MultiplayerEngine + 'static> {
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
pub struct HordeClientTcp<ME:MultiplayerEngine + 'static> {
    adress:(Ipv4Addr, u16),
    events_sender:Sender<Vec<u8>>,
    decoded_events:Receiver<HordeMultiplayerPacket<ME>>,
    id_generator:ME::RIDG,
}

impl<ME:MultiplayerEngine + 'static> HordeClientTcp<ME> {
    fn new(adress:(Ipv4Addr, u16), name:String) -> (Self, usize, usize) {
        let mut stream = TcpStream::connect_timeout(&adress.into(), Duration::from_secs(3)).unwrap();
        let mut buffer = Vec::with_capacity(1024);
        HordeMultiplayerPacket::<ME>::WannaJoin(name).add_bytes(&mut buffer);
        stream.write(&buffer).expect("Got an error while sending for handshake");
        buffer.clear();

        println!("[Multiplayer client] Started handshake");
        let mut id = 0;
        let mut tickrate = 0;
        let mut given_id = false;

        stream.set_read_timeout(Some(Duration::from_secs_f64((1.0/(50.0 as f64)) * 0.5)));
        let mut local_tcp_buffer = vec![0 ; 1024];
        let mut local_decode_buffer = Vec::with_capacity(1024);
        let mut local_decoder = HordeMultiplayerPacket::<ME>::get_decoder();
        while !given_id {
            println!("[Multiplayer client] Reading events from server");
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

        stream.set_read_timeout(Some(Duration::from_secs_f64((1.0/(tickrate as f64)) * 0.5)));
        println!("[Multiplayer client] got player ID");
        buffer.clear();
        HordeMultiplayerPacket::<ME>::SendMeEverything.add_bytes(&mut buffer);
        stream.write(&mut buffer);
        println!("[Multiplayer client] Sent everything request");
        let (events_sender, decoded_events) = TcpStreamHandler::initiate(stream, vec![0 ; 4096], local_decode_buffer, local_decoder, tickrate);
        (Self {id_generator:ME::get_random_id_generator(),adress, events_sender, decoded_events}, id, tickrate)
    }
    fn send_packet(&mut self, packet:HordeMultiplayerPacket<ME>) {
        self.events_sender.send(packet.get_bytes_vec()).unwrap();
    }
    fn get_response_to(&mut self, packet:HordeMultiplayerPacket<ME>, players:&mut HordePlayers<ME::ID>, engine:&mut ME, client_ids:&Vec<ME::ID>) -> Vec<HordeMultiplayerPacket<ME>> {
        //println!("[Multiplayer client] getting a response to a packet");
        let mut response = Vec::with_capacity(4);
        match packet {
            HordeMultiplayerPacket::Chat { from_player, text } => println!("{} : {}", from_player, text),
            HordeMultiplayerPacket::DoYouAgree(_) => panic!("Server sent agree packet, impossible"),
            HordeMultiplayerPacket::PlayerJoined(player) => players.players.write().unwrap().push(player),
            HordeMultiplayerPacket::ResetComponent { id, data } => {
                //println!("[Multiplayer client] receiving component reset event");
                if !client_ids.contains(&id) {
                    engine.set_component(id, data);
                }
            },
            HordeMultiplayerPacket::ResetWorld { wd } => {
                //println!("[Multiplayer client] receiving reset world event");
                engine.set_world(wd);
            },
            HordeMultiplayerPacket::SpreadEvent(evt) => {

                //println!("[Multiplayer client] receiving spread event event");
                if !client_ids.contains(&ME::get_target(&evt)) {
                    engine.apply_event(evt);
                }
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
        for id in &client_ids {
            for compo in engine.get_components_to_sync_for(id) {
                self.send_packet(HordeMultiplayerPacket::ResetComponent { id:id.clone(), data: compo });
            }
        }
        while let Ok(packet) = self.decoded_events.try_recv() {
            let response = self.get_response_to(packet, players, engine, &client_ids);
            for resp in response {
                self.send_packet(resp);
            }
        }
        
        for i in 0..(engine.get_total_len()/50 + 1) {
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

