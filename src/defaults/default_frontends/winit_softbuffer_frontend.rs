use softbuffer::Buffer;
use winit::{application::ApplicationHandler, dpi::Size, event_loop::{ActiveEventLoop, EventLoop}, raw_window_handle::{DisplayHandle, WindowHandle}, window::WindowAttributes, window::Window};

pub struct SoftbufferWindow<'a> {
    buffer:Buffer<'a, DisplayHandle<'a>, WindowHandle<'a>>
}

pub fn window_test() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop.create_window(WindowAttributes::default()).unwrap();
    
}