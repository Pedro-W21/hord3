use std::sync::atomic::Ordering;

use minifb::{Key, Window, WindowOptions};

use crate::horde::{frontend::{interact::Button, HordeWindow, HordeWindowDimensions, MouseState, WindowingEvent, WindowingEventVariant}, rendering::framebuffer::HordeColorFormat};

pub struct MiniFBWindow {
    window:Window,
    u32_buf:Vec<u32>,
    dims:HordeWindowDimensions
}

impl HordeWindow for MiniFBWindow {
    fn new(dims:crate::horde::frontend::HordeWindowDimensions) -> Self {
        let mut options = WindowOptions::default();
        let mut window = Window::new("Horde MiniFB window", dims.get_width(), dims.get_height(), options).expect("Couldn't create window !");
        Self { window, u32_buf:vec![0 ; dims.get_width() * dims.get_height()], dims }
    }
    fn get_events(&mut self) -> Vec<crate::horde::frontend::WindowingEvent> {
        let mut events = Vec::with_capacity(10);
        for key in self.window.get_keys_pressed(minifb::KeyRepeat::Yes) {
            let mut button_pressed = match key {
                Key::A => Button::A,
                Key::B => Button::B,
                Key::C => Button::C,
                Key::D => Button::D,
                Key::E => Button::E,
                Key::F => Button::F,
                Key::G => Button::G,
                Key::H => Button::H,
                Key::I => Button::I,
                Key::J => Button::J,
                Key::K => Button::K,
                Key::L => Button::L,
                Key::M => Button::M,
                Key::N => Button::N,
                Key::O => Button::O,
                Key::P => Button::P,
                Key::Q => Button::Q,
                Key::R => Button::R,
                Key::S => Button::S,
                Key::T => Button::T,
                Key::U => Button::U,
                Key::V => Button::V,
                Key::W => Button::W,
                Key::X => Button::X,
                Key::Y => Button::Y,
                Key::Z => Button::Z,
                Key::Key1 => Button::One,
                Key::Key2 => Button::Two,
                Key::Key3 => Button::Three,
                Key::Key4 => Button::Four,
                Key::Key5 => Button::Five,
                Key::Key6 => Button::Six,
                Key::Key7 => Button::Seven,
                Key::Key8 => Button::Eight,
                Key::Key9 => Button::Nine,
                Key::Key0 => Button::Zero,
                Key::Space => Button::SpaceBar,
                Key::LeftShift => Button::LShift,
                Key::RightShift => Button::RShift,
                Key::LeftCtrl => Button::Ctrl,
                Key::Tab => Button::Tab,
                Key::Escape => Button::Escape,
                Key::NumPadPlus => Button::Plus,
                Key::Minus => Button::Minus,
                _ => Button::RShift
            };
            events.push(WindowingEvent::new(WindowingEventVariant::KeyPress(button_pressed)));
            
        }
        events
    }
    fn present(&mut self) {
        ()
    }
    fn use_framebuffer<'a>(&mut self, framebuf:&mut std::sync::RwLockWriteGuard<'a, crate::horde::frontend::HordeFramebuffer>) {
        match framebuf.get_format() {
            HordeColorFormat::ARGB8888 => {
                assert_eq!(self.u32_buf.len(), self.dims.get_height() * self.dims.get_width());
                assert_eq!(framebuf.get_data().len(), self.dims.get_height() * self.dims.get_width());
                self.u32_buf.copy_from_slice(&framebuf.get_data());
            },
            _ => todo!("Implement other color formats for minifb frontend")
        }
        self.window.update_with_buffer(&self.u32_buf, self.dims.get_width(), self.dims.get_height()).unwrap();
    }
    fn change_mouse_state(&mut self, mouse:MouseState) {
        match self.window.get_mouse_pos(minifb::MouseMode::Pass) {
            Some(mouse_pos) => {
                mouse.get_global_state().x.store(mouse_pos.0 as i32, Ordering::Relaxed);
                mouse.get_global_state().y.store(mouse_pos.1 as i32, Ordering::Relaxed);
            },
            None => ()
        }
        if self.window.get_mouse_down(minifb::MouseButton::Left) {
            mouse.get_global_state().left.store(2, Ordering::Relaxed);
        }
        else {
            mouse.get_global_state().left.store(0, Ordering::Relaxed);
        }
        if self.window.get_mouse_down(minifb::MouseButton::Right) {
            mouse.get_global_state().right.store(2, Ordering::Relaxed);
        }
        else {
            mouse.get_global_state().right.store(0, Ordering::Relaxed);
        }
    }
}