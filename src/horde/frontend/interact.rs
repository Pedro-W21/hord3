use std::collections::HashSet;

use crate::horde::rendering::framebuffer::FrameBufferDimensions;


const ALL_BUTTONS: [Button; 47] = [
    Button::A,
    Button::B,
    Button::C,
    Button::D,
    Button::E,
    Button::F,
    Button::G,
    Button::H,
    Button::I,
    Button::J,
    Button::K,
    Button::L,
    Button::M,
    Button::N,
    Button::O,
    Button::P,
    Button::Q,
    Button::R,
    Button::S,
    Button::T,
    Button::U,
    Button::V,
    Button::W,
    Button::X,
    Button::Y,
    Button::Z,
    Button::One,
    Button::Two,
    Button::Three,
    Button::Four,
    Button::Five,
    Button::Six,
    Button::Seven,
    Button::Eight,
    Button::Nine,
    Button::Zero,
    Button::MiddleClick,
    Button::LeftClick,
    Button::RightClick,
    Button::SpaceBar,
    Button::LShift,
    Button::RShift,
    Button::Ctrl,
    Button::Tab,
    Button::Escape,
    Button::Plus,
    Button::Minus
];

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Button {
    A,
    Z,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    Q,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    M,
    W,
    X,
    C,
    V,
    B,
    N,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Zero,
    LeftClick,
    RightClick,
    MiddleClick,
    SpaceBar,
    LShift,
    RShift,
    Ctrl,
    Tab,
    Escape,
    Plus,
    Minus
}
#[derive(Clone)]
pub struct MouseReport {
    pub x: i32,
    pub y: i32,
    pub scroll: i16,
    pub sensitivity: f32,
}

impl MouseReport {
    pub fn new(sens: f32) -> Self {
        Self {
            x: 0,
            y: 0,
            scroll: 0,
            sensitivity: sens,
        }
    }
    pub fn update(&mut self, x:i32, y:i32, scroll: i16) {
        self.x = x;
        self.y = y;
        self.scroll = scroll;
    }
    pub fn get_centered_screen_coords<FBD:FrameBufferDimensions>(&self, screen_dims:&FBD) -> (f32, f32) {
        (
            (self.x - screen_dims.get_width() as i32 / 2) as f32,
            (self.y - screen_dims.get_height() as i32 / 2) as f32,
        )
    }
    pub fn get_orient_change<FBD:FrameBufferDimensions>(&self, screen_dims:&FBD) -> (f32, f32) {
        let (x, y) = self.get_centered_screen_coords(screen_dims);
        (
            -((x / (screen_dims.get_width() as f32 / 2.0)) * self.sensitivity * 45.0),
            -((y / (screen_dims.get_height() as f32 / 2.0)) * self.sensitivity * 45.0 * 1.0 / screen_dims.get_aspect_ratio()),
        )
    }
}

pub struct ButtonReportParameters {
    forward: Button,
    backward: Button,
    left: Button,
    right: Button,
    up: Button,
    down: Button,
    jump: Button,
    crouch: Button,
    drift: Button,

    action1: Button,
    action2: Button,
    action3: Button,

    attack: Button,
    parry: Button,
    special: Button,

    camera:Button,
    ui:Button,
}

impl ButtonReportParameters {
    pub fn new_simple() -> ButtonReportParameters {
        ButtonReportParameters {
            forward: Button::W,
            backward: Button::S,
            left: Button::A,
            right: Button::D,
            up: Button::C,
            down: Button::V,
            jump: Button::SpaceBar,
            crouch: Button::LShift,
            drift: Button::LShift,
            action1: Button::R,
            action2: Button::E,
            action3: Button::G,
            attack: Button::LeftClick,
            parry: Button::RightClick,
            special: Button::F,
            ui:Button::U,
            camera:Button::I,
        }
    }
}
#[derive(Clone, Copy)]
pub struct ActionReport {
    attack: bool,
    parry: bool,
    special: bool,

    action1: bool,
    action2: bool,
    action3: bool,
}

impl ActionReport {
    pub fn new() -> Self {
        Self {
            attack: false,
            parry: false,
            special: false,
            action1: false,
            action2: false,
            action3: false,
        }
    }
    pub fn reset(&mut self) {
        self.action1 = false;
        self.action2 = false;
        self.action3 = false;
        self.attack = false;
        self.parry = false;
        self.special = false;
    }
}

impl MovementReport {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            jump: false,
            crouch: false,
            drift: false,
        }
    }
    pub fn reset(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.down = false;
        self.jump = false;
        self.crouch = false;
        self.drift = false
    }
}
#[derive(Clone, Copy)]
pub struct MovementReport {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    jump: bool,
    crouch: bool,
    drift: bool,
}

pub struct ButtonReport {
    pressed_buttons: HashSet<Button>,
    previous_pressed_buttons: HashSet<Button>,
    parameters: ButtonReportParameters,
    current_movement: MovementReport,
    old_movement: MovementReport,
    current_action: ActionReport,
    old_action: ActionReport,
}

impl ButtonReport {
    pub fn new() -> ButtonReport {
        ButtonReport {
            pressed_buttons: HashSet::new(),
            previous_pressed_buttons: HashSet::new(),
            parameters: ButtonReportParameters::new_simple(),
            old_action: ActionReport::new(),
            current_action: ActionReport::new(),
            old_movement: MovementReport::new(),
            current_movement: MovementReport::new(),
        }
    }
    pub fn is_pressed(&self, button: Button) -> bool {
        self.pressed_buttons.contains(&button)
    }
    pub fn is_newly_pressed(&self, button: Button) -> bool {
        self.pressed_buttons.contains(&button) && !self.previous_pressed_buttons.contains(&button)
    }
    pub fn isnt_pressed_anymore(&self, button: Button) -> bool {
        !self.pressed_buttons.contains(&button) && self.previous_pressed_buttons.contains(&button)
    }
    pub fn get_all_pressed_buttons(&self) -> Vec<Button> {
        let mut pressed = Vec::new();
        for button in ALL_BUTTONS {
            if self.is_pressed(button) {
                pressed.push(button);
            }
        }
        pressed
    }
    pub fn get_all_newly_pressed_buttons(&self) -> Vec<Button> {
        let mut pressed = Vec::new();
        for button in ALL_BUTTONS {
            if self.is_newly_pressed(button) {
                pressed.push(button);
            }
        }
        pressed
    }
    pub fn get_all_stopped_presses(&self) -> Vec<Button> {
        let mut pressed = Vec::new();
        for button in ALL_BUTTONS {
            if self.isnt_pressed_anymore(button) {
                pressed.push(button);
            }
        }
        pressed
    }
    pub fn update_action_report(&mut self) {
        self.old_action = self.current_action;
        self.current_action.reset();
        if self.is_pressed(self.parameters.action1) {
            self.current_action.action1 = true;
        }
        if self.is_pressed(self.parameters.action2) {
            self.current_action.action2 = true;
        }
        if self.is_pressed(self.parameters.action3) {
            self.current_action.action3 = true;
        }
        if self.is_pressed(self.parameters.attack) {
            self.current_action.attack = true;
        }
        if self.is_pressed(self.parameters.parry) {
            self.current_action.parry = true;
        }
        if self.is_pressed(self.parameters.special) {
            self.current_action.special = true;
        }
    }
    pub fn update_movement_report(&mut self) {
        self.old_movement = self.current_movement;
        self.current_movement.reset();
        if self.is_pressed(self.parameters.forward) {
            self.current_movement.forward = true;
        }
        if self.is_pressed(self.parameters.backward) {
            self.current_movement.backward = true;
        }
        if self.is_pressed(self.parameters.left) {
            self.current_movement.left = true;
        }
        if self.is_pressed(self.parameters.right) {
            self.current_movement.right = true;
        }
        if self.is_pressed(self.parameters.up) {
            self.current_movement.up = true;
        }
        if self.is_pressed(self.parameters.down) {
            self.current_movement.down = true;
        }
        if self.is_pressed(self.parameters.drift) {
            self.current_movement.drift = true;
        }
        if self.is_pressed(self.parameters.jump) {
            self.current_movement.jump = true;
        }
    }
    pub fn update_report(&mut self) {
        self.previous_pressed_buttons = self.pressed_buttons.clone();
        self.pressed_buttons.clear();
        todo!("mmdsmmmdm keyboard");
        self.update_action_report();
        self.update_movement_report();
    }
}

pub enum Focus {
    NoFocus,
    Window,
    UIFocus,
    CameraControl,
}