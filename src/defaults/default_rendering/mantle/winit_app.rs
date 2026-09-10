// Welcome to the instancing example!
//
// This is a simple, modified version of the `triangle.rs` example that demonstrates how we can use
// the "instancing" technique with vulkano to draw many instances of the triangle.

use std::{error::Error, sync::{Arc, RwLock, atomic::{AtomicUsize, Ordering}, mpmc::{Receiver, Sender, channel}}};
use foldhash::HashSet;
use smallvec::smallvec;
use vulkano::{
    Validated, VulkanError, VulkanLibrary, buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, DrawIndexedIndirectCommand, RenderPassBeginInfo, allocator::StandardCommandBufferAllocator,
    }, descriptor_set::{CopyDescriptorSet, DescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator}, device::{
        Device, DeviceCreateInfo, DeviceExtensions, DeviceOwned, Queue, QueueCreateInfo, QueueFlags, physical::PhysicalDeviceType,
    }, format::Format, image::{Image, ImageCreateInfo, ImageType, ImageUsage, sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo}, view::ImageView}, instance::{Instance, InstanceCreateFlags, InstanceCreateInfo}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator}, pipeline::{
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo, graphics::{
            GraphicsPipelineCreateInfo, color_blend::{ColorBlendAttachmentState, ColorBlendState}, depth_stencil::{DepthState, DepthStencilState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::{Viewport, ViewportState},
        }, layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateInfo},
    }, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, single_pass_renderpass, swapchain::{
        Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
    }, sync::{self, GpuFuture},
};
use winit::{
    application::ApplicationHandler, event::{KeyEvent, MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{Key, NamedKey}, window::{Window, WindowId},
};

use crate::{defaults::default_rendering::mantle::{api::{MantleEvent, MantleHandler, MantleResponse}, meshes::{CameraData, InstanceData, MemoryAllocator, Meshes, TriangleVertex}, textures::TextureAtlas}, horde::{frontend::{MouseState, WindowingEvent, interact::Button}, geometry::{mat4::Mat4, vec3d::Vec3Df}, rendering::camera::Camera}};

fn main() -> Result<(), impl Error> {
    let event_loop = EventLoop::new().unwrap();
    let (mut app,handler) = App::new(&event_loop);

    event_loop.run_app(&mut app)
}

pub struct App {
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    rcx: Option<RenderContext>,
    memory_allocator:MemoryAllocator,
    meshes:Arc<RwLock<Meshes>>,
    textures:Arc<RwLock<TextureAtlas>>,
    events:Receiver<MantleEvent>,
    window_events:Sender<WindowingEvent>,
    mouse_state:MouseState
}

pub struct RenderContext {
    window: Arc<Window>,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> (Self, MantleHandler) {
        let library = unsafe { VulkanLibrary::new() }.unwrap();
        let required_extensions = Surface::required_extensions(event_loop).unwrap();
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.presentation_support(i as u32, event_loop).unwrap()
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .unwrap();

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let memory_allocator: Arc<vulkano::memory::allocator::GenericMemoryAllocator<vulkano::memory::allocator::FreeListAllocator>> = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let (sender, receiver) = channel();
        let (sender2, receiver2) = channel();

        let (sender3, receiver3) = channel();
        let mouse_state = MouseState::new();
        let handler = MantleHandler::new(sender, receiver2, mouse_state.clone(), receiver3);
        (
            App {
                instance,
                device,
                queue,
                command_buffer_allocator,
                meshes:Arc::new(RwLock::new(Meshes { meshes: vec![], allocator:memory_allocator.clone(), mesh_creation_sender:sender2, camera:Camera::empty() })),
                textures:Arc::new(RwLock::new(TextureAtlas::new(4096, 4096, 1, memory_allocator.clone()))),
                rcx: None,
                events:receiver,
                memory_allocator,
                window_events:sender3,
                mouse_state
            },
            handler
        )
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let surface = Surface::from_window(self.instance.clone(), window.clone()).unwrap();
        let window_size = window.clone().inner_size();

        let (swapchain, images) = {
            let surface_capabilities = self
                .device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let (image_format, _) = self
                .device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0];

            Swapchain::new(
                self.device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window_size.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let render_pass = single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth: {
                    format: vulkano::format::Format::D32_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth},
            },
        )
        .unwrap();

        let framebuffers = window_size_dependent_setup(self.memory_allocator.clone(), &images, &render_pass);

        mod vs {
            vulkano_shaders::shader! {
                ty: "vertex",
                src: r"
                    #version 450

                    // The triangle vertex positions.
                    layout(location = 0) in vec3 position;
                    layout(location = 1) in vec2 uv;

                    // The per-instance data.
                    layout(location = 2) in vec3 world_position;
                    layout(location = 3) in float scale;

                    // Camera Data (Uniform Buffer)
                    layout(set = 0, binding = 0) uniform CameraBuffer {
                        mat4 view;
                        mat4 projection;
                    } camera;

                    layout(location = 0) out vec2 v_uv;

                    void main() {
                        // Apply the scale and offset for the instance.
                        vec3 worldspace = position * scale + world_position;

                        vec4 cameraspace = camera.view * vec4(worldspace, 1.0);

                        gl_Position = camera.projection * cameraspace;
                        v_uv = uv;
                    }
                ",
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                src: r"
                    #version 450

                    layout(location = 0) in vec2 v_uv;

                    layout(location = 0) out vec4 f_color;

                    layout(set = 1, binding = 0) uniform sampler2D u_texture_atlas;

                    void main() {
                        f_color = texture(u_texture_atlas, v_uv);
                    }
                ",
            }
        }

        let pipeline = {
            let vs = unsafe { vs::load(self.device.clone()) }
                .unwrap()
                .entry_point("main")
                .unwrap();
            let fs = unsafe { fs::load(self.device.clone()) }
                .unwrap()
                .entry_point("main")
                .unwrap();
            let vertex_input_state = [TriangleVertex::per_vertex(), InstanceData::per_instance()]
                .definition(&vs)
                .unwrap();
            let stages = smallvec![
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(self.device.clone(), PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages).into_pipeline_layout_create_info(self.device.clone()).unwrap()).unwrap();
            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::new(
                self.device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages,
                    // Use the implementations of the `Vertex` trait to describe to vulkano how the
                    // two vertex types are expected to be used.
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState {
                        attachments: vec![ColorBlendAttachmentState::default()],
                        ..Default::default()
                    }),
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState::simple()),
                        ..Default::default()
                    }),
                    dynamic_state: HashSet::from_iter([DynamicState::Viewport]),
                    subpass: Some((subpass).into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .unwrap()
        };

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window_size.into(),
            depth_range:0.0..=1.0,
        };

        let previous_frame_end = Some(sync::now(self.device.clone()).boxed());

        self.rcx = Some(RenderContext {
            window,
            swapchain,
            render_pass,
            framebuffers,
            pipeline,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let rcx = self.rcx.as_mut().unwrap();

        match event {
            WindowEvent::CursorMoved { device_id, position } => {
                self.mouse_state.get_global_state().x.store(position.x as i32, Ordering::Relaxed);
                self.mouse_state.get_global_state().y.store(position.y as i32, Ordering::Relaxed);
            }
            WindowEvent::MouseInput { device_id, state, button } => {
                match button {
                    MouseButton::Left => self.mouse_state.get_global_state().left.store(if state.is_pressed() {2} else {0}, Ordering::Relaxed),
                    MouseButton::Right => self.mouse_state.get_global_state().right.store(if state.is_pressed() {2} else {0}, Ordering::Relaxed),
                    _ => ()
                }
            }
            WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                let key = match event.logical_key {
                    Key::Named(key) => match key {
                        NamedKey::Space => Some(Button::SpaceBar),
                        NamedKey::Shift => Some(Button::LShift),
                        NamedKey::Control => Some(Button::Ctrl),
                        NamedKey::Tab => Some(Button::Tab),
                        NamedKey::Escape => Some(Button::Escape),
                        _ => None
                    },
                    Key::Character(chara) => Some(match chara.as_str().to_lowercase().as_str() {
                        "a" => Button::A,
                        "b" => Button::B,
                        "c" => Button::C,
                        "d" => Button::D,
                        "e" => Button::E,
                        "f" => Button::F,
                        "g" => Button::G,
                        "h" => Button::H,
                        "i" => Button::I,
                        "j" => Button::J,
                        "k" => Button::K,
                        "l" => Button::L,
                        "m" => Button::M,
                        "n" => Button::N,
                        "o" => Button::O,
                        "p" => Button::P,
                        "q" => Button::Q,
                        "r" => Button::R,
                        "s" => Button::S,
                        "t" => Button::T,
                        "u" => Button::U,
                        "v" => Button::V,
                        "w" => Button::W,
                        "x" => Button::X,
                        "y" => Button::Y,
                        "z" => Button::Z,

                        "0" => Button::Zero,
                        "1" => Button::One,
                        "2" => Button::Two,
                        "3" => Button::Three,
                        "4" => Button::Four,
                        "5" => Button::Five,
                        "6" => Button::Six,
                        "7" => Button::Seven,
                        "8" => Button::Eight,
                        "9" => Button::Nine,
                        _ => Button::RShift
                    }),
                    Key::Unidentified(_) | Key::Dead(_) => None
                };
                if let Some(button) = key {
                    if event.state.is_pressed() {
                        self.window_events.send(WindowingEvent::new(crate::horde::frontend::WindowingEventVariant::KeyPress(button))).unwrap();
                    }
                    else {
                        self.window_events.send(WindowingEvent::new(crate::horde::frontend::WindowingEventVariant::KeyRelease(button))).unwrap();
                    }
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                rcx.recreate_swapchain = true;
            }
            WindowEvent::RedrawRequested => {
                
                let window_size = rcx.window.inner_size();

                if window_size.width == 0 || window_size.height == 0 {
                    return;
                }
                if let Some(mut future) = rcx.previous_frame_end.take() {
                    future.cleanup_finished();
                    future.flush().unwrap(); 
                }

                if rcx.recreate_swapchain {
                    let (new_swapchain, new_images) = rcx
                        .swapchain
                        .recreate(SwapchainCreateInfo {
                            image_extent: window_size.into(),
                            ..rcx.swapchain.create_info()
                        })
                        .expect("failed to recreate swapchain");

                    rcx.swapchain = new_swapchain;
                    rcx.framebuffers = window_size_dependent_setup(self.memory_allocator.clone(), &new_images, &rcx.render_pass);
                    rcx.viewport.extent = window_size.into();
                    rcx.recreate_swapchain = false;
                }

                let (image_index, suboptimal, acquire_future) = match acquire_next_image(
                    rcx.swapchain.clone(),
                    None,
                )
                .map_err(Validated::unwrap)
                {
                    Ok(r) => r,
                    Err(VulkanError::OutOfDate) => {
                        rcx.recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("failed to acquire next image: {e}"),
                };

                if suboptimal {
                    rcx.recreate_swapchain = true;
                }



                let mut builder  = AutoCommandBufferBuilder::primary(
                    self.command_buffer_allocator.clone(),
                    self.queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                let aspect_ratio = (window_size.width as f32)/(window_size.height as f32);
                let (camera, atlas) = {
                    let mut meshes = self.meshes.write().unwrap();
                    let mut textures = self.textures.write().unwrap();
                    while let Ok(event) = self.events.try_recv() {
                        meshes.apply_event(event,&mut textures, &mut builder);
                    }
                    (meshes.get_new_camdata(aspect_ratio), textures.get_image())
                };



                let descriptor_set_allocator =
                    StandardDescriptorSetAllocator::new(self.device.clone(), Default::default());
                let pipeline_layout = rcx.pipeline.layout();
                let descriptor_set_layouts = pipeline_layout.set_layouts();

                let descriptor_set_layout_index = 0;
                let descriptor_set_layout = descriptor_set_layouts
                    .get(descriptor_set_layout_index)
                    .unwrap();
                                
                
                let buffer = Buffer::from_data(
                    self.memory_allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::UNIFORM_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    camera
                ).unwrap();



                let descriptor_set_allocator = Arc::new(descriptor_set_allocator);
                let camera_descriptor_set = DescriptorSet::new(
                    descriptor_set_allocator.clone(),
                    descriptor_set_layout.clone(),
                    [WriteDescriptorSet::buffer(0, buffer)], // 0 is the binding
                    [],
                )
                .unwrap();

                let image_view = ImageView::new_default(atlas).unwrap();

                let sampler = Sampler::new(
                    image_view.device().clone(),
                    SamplerCreateInfo {
                        mag_filter: Filter::Nearest, // Pratique pour du pixel art / atlases
                        min_filter: Filter::Nearest,
                        address_mode: [SamplerAddressMode::ClampToEdge; 3],
                        ..Default::default()
                    },
                )
                .unwrap();


                let descriptor_set_layout_index = 1;
                let descriptor_set_layout = descriptor_set_layouts
                    .get(descriptor_set_layout_index)
                    .unwrap();


                let atlas_descriptor_set = DescriptorSet::new(
                    descriptor_set_allocator.clone(),
                    descriptor_set_layout.clone(),
                    [WriteDescriptorSet::image_view_sampler(0, image_view, sampler)], // 0 is the binding
                    [],
                )
                .unwrap();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![
                                Some([0.0, 0.0, 1.0, 1.0].into()),
                                Some(1.0f32.into())
                                ],
                            ..RenderPassBeginInfo::framebuffer(
                                rcx.framebuffers[image_index as usize].clone(),
                            )
                        },
                        Default::default(),
                    )
                    .unwrap()
                    .set_viewport(0, [rcx.viewport.clone()].into_iter().collect())
                    .unwrap()
                    .bind_pipeline_graphics(rcx.pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(vulkano::pipeline::PipelineBindPoint::Graphics,
                        pipeline_layout.clone(),
                        0,
                        (camera_descriptor_set, atlas_descriptor_set)
                    ).unwrap();
            
                for mesh in &self.meshes.read().unwrap().meshes {
                    if mesh.show {
                        // We pass both our lists of vertices here.
                        let lod = &mesh.lods[0];
                        builder.bind_vertex_buffers(
                            0,
                            (lod.vertex_buffer.clone(), mesh.instances.instance_buffer.clone()),
                        )
                        .unwrap();
                        builder.bind_index_buffer(
                            lod.indices.clone()
                        )
                        .unwrap();

                        unsafe {
                            builder.draw_indexed(
                                lod.vertex_buffer.len() as u32,
                                mesh.instances.instance_buffer.len() as u32,
                                0,
                                0,
                                0
                            )
                        }
                        .unwrap();
                    }
                    
                }

                builder.end_render_pass(Default::default()).unwrap();

                let command_buffer = builder.build().unwrap();
                let future = acquire_future
                    .then_execute(self.queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(
                        self.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(rcx.swapchain.clone(), image_index),
                    )
                    .then_signal_fence_and_flush();

                match future.map_err(Validated::unwrap) {
                    Ok(future) => {
                        rcx.previous_frame_end = Some(future.boxed());
                    }
                    Err(VulkanError::OutOfDate) => {
                        rcx.recreate_swapchain = true;
                        rcx.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        rcx.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let rcx = self.rcx.as_mut().unwrap();
        rcx.window.request_redraw();
    }
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(
    allocator:MemoryAllocator,
    images: &[Arc<Image>],
    render_pass: &Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    let extent = images[0].extent();

    let depth_image = Image::new(
        allocator,
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::D32_SFLOAT, 
            extent,
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )
    .unwrap();

    let depth_view = ImageView::new_default(depth_image).unwrap();
    images
        .iter()
        .map(|image| {
            
            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_view.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}