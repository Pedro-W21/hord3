use std::{cell::SyncUnsafeCell, sync::{atomic::{AtomicBool, AtomicI32, AtomicI8, Ordering}, Arc, RwLock, RwLockWriteGuard}, thread, time::{Duration, Instant}};

use crossbeam::channel::{unbounded, Receiver, Sender};
use interact::Button;

use super::{rendering::framebuffer::HordeColorFormat, scheduler::IndividualTask};

pub mod interact;

pub trait HordeFrontend {
    
}

#[derive(Clone)]
pub struct WindowingEvent {
    variant:WindowingEventVariant,
    time:Instant,
}

impl WindowingEvent {
    pub fn new(variant:WindowingEventVariant) -> Self {
        Self { variant, time: Instant::now() }
    }
    pub fn get_variant(self) -> WindowingEventVariant {
        self.variant
    }
}
#[derive(Clone)]
pub enum WindowingEventVariant {
    KeyPress(Button),
    KeyRelease(Button),
    DoneWithFramebuffer,
    DoneWithEvents
}

pub enum WindowingOrder {
    Present,
    Stop,
    GetAllEventsAndMouse
}

pub struct HordeFramebuffer {
    data:Vec<u32>,
    color_format:HordeColorFormat,
    dimensions:HordeWindowDimensions,
}

impl HordeFramebuffer {
    pub fn new(dims:HordeWindowDimensions, format:HordeColorFormat) -> Self {
        match format {
            HordeColorFormat::RGB888 => Self {color_format:format, data:vec![0 ; dims.width * dims.height], dimensions:dims},
            HordeColorFormat::RGBA8888 | HordeColorFormat::ARGB8888 => Self {color_format:format, data:vec![0 ; dims.width * dims.height], dimensions:dims},
        }
    }
    pub fn get_data(&mut self) -> &mut Vec<u32> {
        &mut self.data
    }
    pub fn get_format(&self) -> HordeColorFormat {
        self.color_format
    }
    pub fn get_dims(&self) -> HordeWindowDimensions {
        self.dimensions
    }
}

#[derive(Clone)]
pub struct WindowingHandler {
    framebuffer:Arc<RwLock<HordeFramebuffer>>,
    outside_framebuffer:Arc<RwLock<SyncUnsafeHordeFramebuffer>>,
    done_with_framebuffer:Arc<AtomicBool>,
    send_order:Sender<WindowingOrder>,
    rcv_event:Receiver<WindowingEvent>,
    outside_events_send:Sender<WindowingEvent>,
    rcv_outside_events:Receiver<WindowingEvent>,
    mouse_pos:MouseState
}

pub struct GlobalMouseState {
    pub x:AtomicI32,
    pub y:AtomicI32,
    pub left:AtomicI8,
    pub right:AtomicI8,
    pub scroll:AtomicI32
}

#[derive(Clone, Debug)]
pub struct LocalMouseState {
    pub x:i32,
    pub y:i32,
    pub left:i8,
    pub right:i8,
    pub scroll:i32,
}

#[derive(Clone)]
pub struct MouseState {
    global_state:Arc<GlobalMouseState>,
    current_state:LocalMouseState,
    previous_state:LocalMouseState
}

impl MouseState {
    pub fn new() -> MouseState {
        Self {
            global_state: Arc::new(GlobalMouseState {
                x:AtomicI32::new(0),
                y:AtomicI32::new(0),
                left:AtomicI8::new(0),
                right:AtomicI8::new(0),
                scroll:AtomicI32::new(0)
            }),
            current_state: LocalMouseState {
                x:0,
                y:0,
                left:0,
                right:0,
                scroll:0
            },
            previous_state: LocalMouseState {
                x:0,
                y:0,
                left:0,
                right:0,
                scroll:0
            }
        }
    }
    pub fn update_local(&mut self) {
        self.previous_state = self.current_state.clone();
        self.current_state.x = self.global_state.x.load(Ordering::Relaxed);
        self.current_state.y = self.global_state.y.load(Ordering::Relaxed);
        self.current_state.left = self.global_state.left.load(Ordering::Relaxed);
        self.current_state.right = self.global_state.right.load(Ordering::Relaxed);
        self.current_state.scroll = self.global_state.scroll.load(Ordering::Relaxed);
    }
    pub fn get_current_state(&self) -> LocalMouseState {
        self.current_state.clone()
    }
    pub fn get_previous_state(&self) -> LocalMouseState {
        self.previous_state.clone()
    }
    pub fn get_global_state(&self) -> Arc<GlobalMouseState> {
        self.global_state.clone()
    }
    pub fn get_deltas_and_scroll(&self) -> LocalMouseState {
        LocalMouseState { x: self.current_state.x - self.previous_state.x, y:self.current_state.y - self.previous_state.y, left:self.current_state.left - self.previous_state.left, right: self.current_state.right - self.previous_state.right, scroll: self.current_state.scroll }
    }
}



pub struct SyncUnsafeHordeFramebuffer {
    data:SyncUnsafeCell<Vec<u32>>,
    other_data:SyncUnsafeCell<Vec<u32>>,
    color_format:HordeColorFormat,
    dimensions:HordeWindowDimensions,
    phase:AtomicBool
}

impl SyncUnsafeHordeFramebuffer {
    pub fn new(dims:HordeWindowDimensions, format:HordeColorFormat) -> Self {
        match format {
            HordeColorFormat::RGB888 => Self {phase:AtomicBool::new(false),color_format:format, data:SyncUnsafeCell::new(vec![0 ; dims.width * dims.height]),other_data:SyncUnsafeCell::new(vec![0 ; dims.width * dims.height]), dimensions:dims},
            HordeColorFormat::RGBA8888 | HordeColorFormat::ARGB8888 => Self {phase:AtomicBool::new(false),color_format:format, data:SyncUnsafeCell::new(vec![0 ; dims.width * dims.height]),other_data:SyncUnsafeCell::new(vec![0 ; dims.width * dims.height]), dimensions:dims},
        }
    }
    pub fn get_data(&mut self) -> &mut Vec<u32> {
        if self.phase.load(Ordering::Relaxed) {
            unsafe {
                &mut *self.data.get()
            }
        }
        else {
            unsafe {
                &mut *self.other_data.get()
            }
        }
        
    }
    pub fn get_other_data(&mut self) -> &mut Vec<u32> {
        if self.phase.load(Ordering::Relaxed) {
            unsafe {
                &mut *self.other_data.get()
            }
        }
        else {
            unsafe {
                &mut *self.data.get()
            }
        }
    }
    pub fn change_phase(&self) {
        self.phase.fetch_not(Ordering::Relaxed);
    }
    pub fn get_data_cell(&self) -> &SyncUnsafeCell<Vec<u32>> {
        if self.phase.load(Ordering::Relaxed) {
            &self.data
        }
        else {
            &self.other_data
        }
    }
    pub fn get_other_data_immut(&self) -> &Vec<u32> {
        unsafe {
            if self.phase.load(Ordering::Relaxed) {
                & *self.other_data.get()
            }
            else {
                & *self.data.get()
            }
        }
        
    }
    pub fn get_format(&self) -> HordeColorFormat {
        self.color_format
    }
    pub fn get_dims(&self) -> HordeWindowDimensions {
        self.dimensions
    }
}

pub struct WindowingThread<HW:HordeWindow> {
    framebuffer:Arc<RwLock<HordeFramebuffer>>,
    rcv_order:Receiver<WindowingOrder>,
    send_event:Sender<WindowingEvent>,
    window:HW,
    mouse_pos:MouseState
}

#[derive(Clone, Copy)]
pub struct HordeWindowDimensions {
    width:usize,
    height:usize,
    width_i:isize,
    height_i:isize,
}

impl HordeWindowDimensions {
    pub fn new(width:usize, height:usize) -> Self {
        Self { width, height, width_i:width as isize, height_i:height as isize }
    }
    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_width_i(&self) -> isize {
        self.width_i
    }
    pub fn get_height(&self) -> usize {
        self.height
    }
    pub fn get_height_i(&self) -> isize {
        self.height_i
    }
}

pub trait HordeWindow {
    fn new(dims:HordeWindowDimensions) -> Self;
    fn present(&mut self);
    fn get_events(&mut self) -> Vec<WindowingEvent>;
    fn change_mouse_state(&mut self, mouse:MouseState); 
    fn use_framebuffer<'a>(&mut self, framebuf:&mut RwLockWriteGuard<'a, HordeFramebuffer>);
}

impl WindowingHandler {
    pub fn new<HW:HordeWindow>(dims:HordeWindowDimensions, format:HordeColorFormat) -> Self {
        let (send_order, rcv_order) = unbounded();
        let (send_event, rcv_event) = unbounded();
        let framebuffer = Arc::new(RwLock::new(HordeFramebuffer::new(dims, format)));
        let framebuf_clone = framebuffer.clone();
        let (outside_events_send, rcv_outside_events) = unbounded();
        let mouse_pos = MouseState::new();
        let mouse_pos_clone = mouse_pos.clone();
        thread::spawn(move || {
            let mut windowing_thread = WindowingThread {
                rcv_order,
                send_event,
                framebuffer:framebuf_clone,
                window:HW::new(dims),
                mouse_pos:mouse_pos_clone
            };
            loop {
                match windowing_thread.rcv_order.recv_timeout(Duration::from_millis(100000)).expect("womp womp, message passing error on windowing thread") {
                    WindowingOrder::Present => {windowing_thread.window.use_framebuffer(&mut windowing_thread.framebuffer.write().unwrap()); windowing_thread.send_event.send(WindowingEvent {time:Instant::now(), variant:WindowingEventVariant::DoneWithFramebuffer}); windowing_thread.window.present()},
                    WindowingOrder::Stop => break,
                    WindowingOrder::GetAllEventsAndMouse => {windowing_thread.window.change_mouse_state(windowing_thread.mouse_pos.clone()); windowing_thread.send_event.send(WindowingEvent { variant: WindowingEventVariant::DoneWithEvents, time: Instant::now() }); let mut events = windowing_thread.window.get_events(); events.into_iter().for_each(|evt| {windowing_thread.send_event.send(evt);});}
                }
            }
        });

        Self {mouse_pos, outside_events_send, rcv_outside_events, framebuffer, done_with_framebuffer:Arc::new(AtomicBool::new(true)), send_order, rcv_event, outside_framebuffer:Arc::new(RwLock::new(SyncUnsafeHordeFramebuffer::new(dims, format))) }
    }

    pub fn get_outside_framebuf(&self) -> Arc<RwLock<SyncUnsafeHordeFramebuffer>> {
        self.outside_framebuffer.clone()
    }
    pub fn get_outside_events(&self) -> Receiver<WindowingEvent> {
        self.rcv_outside_events.clone()
    }
    pub fn get_mouse_state(&self) -> MouseState {
        self.mouse_pos.clone()
    }

    fn send_framebuf(&mut self) {
        match self.outside_framebuffer.read() {
            Ok(read) => if self.done_with_framebuffer.load(Ordering::Relaxed) {
                unsafe {
                    self.done_with_framebuffer.store(false, Ordering::Relaxed);
                    self.framebuffer.write().unwrap().data.copy_from_slice(&read.get_other_data_immut());
                    self.send_order.send(WindowingOrder::Present);
                }
                
            },
            Err(_) => panic!("Outside framebuffer still in use"),
        }
    }
    fn wait_for_present(&mut self) {
        loop {
            match self.rcv_event.recv() {
                Ok(evt) => match &evt.variant {
                    WindowingEventVariant::DoneWithFramebuffer => {
                        self.done_with_framebuffer.store(true, Ordering::Relaxed);
                        break;
                    }
                    other => self.outside_events_send.send(evt).unwrap(), 
                },
                Err(_) => panic!("Error receiving window event")
            }
        }
        loop {
            match self.rcv_event.try_recv() {
                Ok(evt) => self.outside_events_send.send(evt).unwrap(),
                Err(_) => break
            }
        }
    }
    fn get_them_events(&self) {
        self.send_order.send(WindowingOrder::GetAllEventsAndMouse);
        loop {
            match self.rcv_event.try_recv() {
                Ok(evt) => 
                match &evt.variant {
                    WindowingEventVariant::DoneWithEvents => break,
                    _ => {
                        self.outside_events_send.send(evt).unwrap();
                    }
                }
                ,
                Err(_) => ()
            }
        }
    }
}

impl IndividualTask for WindowingHandler {
    type TD = usize;
    type TID = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => self.send_framebuf(),
            1 => self.wait_for_present(),
            2 => self.get_them_events(),
            _ => panic!("No task ID beyond 1 for windowing handler")
        }
    }
}