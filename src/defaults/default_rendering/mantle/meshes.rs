use std::{collections::HashMap, f32::consts::PI, sync::{Arc, atomic::{AtomicU64, AtomicUsize}, mpmc::{Receiver, Sender, channel}}};

use to_from_bytes_derive::{FromBytes, ToBytes};
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::CopyBufferInfo, format, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::graphics::vertex_input::Vertex};

use crate::{defaults::default_rendering::mantle::{api::{self, CPUInstanceData, MantleEvent, MantleRequest, MantleResponse}, buffer_allocator::{Allocation, AllocationDemand, BufferAllocator}, textures::TextureAtlas}, horde::{geometry::{mat4::Mat4, rotation::Rotation, vec3d::Vec3Df}, rendering::camera::Camera}};


pub type MemoryAllocator = Arc<vulkano::memory::allocator::GenericMemoryAllocator<vulkano::memory::allocator::FreeListAllocator>>;
pub struct Meshes {
    pub meshes:Vec<Mesh>,
    pub vertex_alloc:BufferAllocator<TriangleVertex>,
    pub index_alloc:BufferAllocator<u32>,
    pub instance_alloc:BufferAllocator<InstanceData>,
    pub mesh_map:HashMap<String, usize>,
    pub mesh_creation_sender:Sender<MantleResponse>,
    pub allocator:MemoryAllocator,
    pub camera:Camera,
}

impl Meshes {
    pub fn apply_event(&mut self, event:MantleEvent, textures:&mut TextureAtlas, builder:&mut vulkano::command_buffer::AutoCommandBufferBuilder<vulkano::command_buffer::PrimaryAutoCommandBuffer>) {
        match event.update {
            MantleRequest::SetGlobalLOD { mesh_id, lod } => {
                self.get_mesh_mut(mesh_id).and_then(|mesh| {mesh.chosen_lod = lod; Some(1_u8)});
                event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
            },
            MantleRequest::CreateInstance { mesh_id, chosen_id, new_data } => {
                let allocator = self.allocator.clone();
                self.get_mesh_mut(mesh_id.clone()).and_then(|mesh| {
                    if mesh.instances.local_instance_buffer.len() < chosen_id {
                        let mut data = mesh.instances.local_instance_buffer.clone();
                        for i in (data.len().max(1) - 1)..chosen_id {
                            data.push(InstanceData { world_position: [new_data.position.x, new_data.position.z, -new_data.position.y], scale: 0.0});
                        }
                        data.last_mut().unwrap().scale = 1.0;

                        mesh.instances.local_instance_buffer = data;
                        mesh.instances.changed_in_frame = true;
                    }
                    else {
                        mesh.instances.local_instance_buffer.get_mut(chosen_id).and_then(|data| {*data = InstanceData { world_position: [new_data.position.x, new_data.position.z, -new_data.position.y], scale: 1.0}; Some(1_u8)});
                        mesh.instances.changed_in_frame = true;
                    }    
                    

                    Some(1_u8)
                });
                event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
            },
            MantleRequest::UpdateInstance { mesh_id, instance, new_data } => {
                self.get_mesh_mut(mesh_id).and_then(|mesh| {
                    mesh.instances.changed_in_frame = true;
                    mesh.instances.local_instance_buffer.get_mut(instance).and_then(|data| {*data = InstanceData { world_position: [new_data.position.x, new_data.position.z, -new_data.position.y], scale: 1.0}; Some(1_u8)});
                    Some(1_u8)
                });
                event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
            },
            MantleRequest::RemoveInstance { mesh_id, removed_id } => (),
            MantleRequest::CreateOrUpdateMesh { name, lods, first_instances } => {
                // println!("[Mantle] : CREATING OR UPDATING MESH {}", name.clone());
                let show = lods[0].index_data.len() > 0;
                if let Some(mesh) = self.get_mesh(MeshID::Name(name.clone())).cloned() {
                    for lod in &mesh.lods {
                        self.vertex_alloc.free(lod.vertex_allocation);
                        self.index_alloc.free(lod.index_allocation);
                    }
                }
                let meshlods:Vec<MeshLOD> = lods.into_iter().map(|apilod| {
                        let added = if apilod.index_data.len() > 0 {
                            100
                        }
                        else {
                            0
                        };
                        let vertex_allocation = self.vertex_alloc.allocate(AllocationDemand::new(apilod.vertex_data.len(), apilod.vertex_data.len() + added));
                        self.vertex_alloc.update_allocation(
                            vertex_allocation,
                            &apilod.vertex_data.iter().map(|vertex| {TriangleVertex {position:[vertex.position.x, vertex.position.z, -vertex.position.y], uv:textures.get_uv_for(apilod.textures[vertex.texture_id as usize].clone(), vertex.u, vertex.v)}}).collect()
                        );

                        let index_allocation = self.index_alloc.allocate(AllocationDemand::new(apilod.index_data.len(), apilod.index_data.len() + added * 3));
                        self.index_alloc.update_allocation(
                            index_allocation,
                            &apilod.index_data.iter().map(|index| {index.vertex}).collect()
                        );
                        MeshLOD { vertex_allocation, index_allocation }
                }).collect();
                if let Some(mesh) = self.get_mesh_mut(MeshID::Name(name.clone())) {
                    
                    mesh.lods = meshlods;
                    mesh.show = show;
                    event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
                }
                else {
                    let id = self.meshes.len();
                    let instances = Instances::new(first_instances, self.allocator.clone(), &mut self.instance_alloc);
                    let id_generator = instances.id_generator.clone();
                    self.meshes.push(Mesh { name:name.clone(), lods:meshlods, instances, chosen_lod: None, show });
                    self.mesh_map.insert(name.clone(), id);
                    self.mesh_creation_sender.send(MantleResponse::MeshCreated { id_generator:id_generator.clone(), direct_id:id, name:name.clone() }).unwrap();
                    event.response.and_then(|resp| {resp.send(MantleResponse::MeshCreated { id_generator, direct_id:id, name });Some(1_u8)});
                }
            },
            MantleRequest::UpdateCamera {
                new_cam
            } => {
                self.camera = new_cam;
                event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
            },
            MantleRequest::CreateOrUpdateTexture { name, texture_data, width, height } => {
                textures.add_or_update_texture(name, texture_data, width, height, builder);
                event.response.and_then(|resp| {resp.send(MantleResponse::Success);Some(1_u8)});
            }
        }
    }
    pub fn push_instance_changes(&mut self, builder:&mut vulkano::command_buffer::AutoCommandBufferBuilder<vulkano::command_buffer::PrimaryAutoCommandBuffer>) {

        for mesh in &mut self.meshes {
            if mesh.instances.changed_in_frame {
                if mesh.instances.allocation.get_length() != mesh.instances.local_instance_buffer.len() {
                    let new_max_length = if mesh.instances.allocation.get_max_length() < mesh.instances.local_instance_buffer.len() {
                        mesh.instances.local_instance_buffer.len() * 3
                    }
                    else {
                        mesh.instances.allocation.get_max_length()
                    };
                    mesh.instances.allocation = self.instance_alloc.extend_allocation(mesh.instances.allocation, AllocationDemand::new(mesh.instances.local_instance_buffer.len(), new_max_length));
                }
                self.instance_alloc.update_allocation(mesh.instances.allocation, &mesh.instances.local_instance_buffer);
                /*let instance_buffer = Buffer::from_iter(
                    self.allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::VERTEX_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    mesh.instances.local_instance_buffer.iter().cloned(),
                )
                .unwrap();
                mesh.instances.instance_buffer = instance_buffer;*/
                mesh.instances.changed_in_frame = false;
            }
        }
    }
    pub fn get_mesh_mut(&mut self, id:MeshID) -> Option<&mut Mesh> {
        match id {
            MeshID::DirectID(index) => self.meshes.get_mut(index),
            MeshID::Name(name) => self.mesh_map.get(&name).and_then(|idx| {Some(self.meshes.get_mut(*idx).unwrap())})
        }
    }
    pub fn get_mesh(&self, id:MeshID) -> Option<&Mesh> {
        match id {
            MeshID::DirectID(index) => self.meshes.get(index),
            MeshID::Name(name) => self.mesh_map.get(&name).and_then(|idx| {Some(self.meshes.get(*idx).unwrap())})
        }
    }
    pub fn get_new_camdata(&self, aspect_ratio:f32) -> CameraData {
        let dir = self.camera.orient.into_vec();
        let view = Mat4::look_to(Vec3Df::new(self.camera.pos.x, self.camera.pos.z, -self.camera.pos.y), Vec3Df::new(dir.x, dir.z, -dir.y), Vec3Df::new(self.camera.orient.roll.sin(), self.camera.orient.roll.cos(), 0.0));
        let projection = Mat4::perspective(PI/3.0, aspect_ratio, 1.0, 1000.0);
        CameraData::from_view_projection(view, projection)
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub name:String,
    pub lods:Vec<MeshLOD>,
    pub instances:Instances,
    pub chosen_lod:Option<usize>,
    pub show:bool
}

#[derive(Clone)]
pub struct Instances {
    pub allocation:Allocation,
    pub local_instance_buffer: Vec<InstanceData>,
    pub changed_in_frame:bool,
    pub id_generator:InstanceIDGenerator
    
}

impl Instances {
    pub fn new(first_instances:Vec<CPUInstanceData>,allocator:MemoryAllocator, instance_alloc:&mut BufferAllocator<InstanceData>) -> Self {
        let mut data = Vec::with_capacity(first_instances.len());
        let highest_id = first_instances.len();
        for instance in first_instances {
            data.push(InstanceData { world_position: [instance.position.x, instance.position.z, -instance.position.y], scale: 1.0});
        }
        if data.len() == 0 {
            data.push(InstanceData { world_position: [0.0, 0.0, 0.0], scale: 0.0 });
        }
        let allocation = instance_alloc.allocate(AllocationDemand::simple(highest_id));
        instance_alloc.update_allocation(allocation, &data);
        Self {
            allocation,
            local_instance_buffer:data,
            changed_in_frame:false,
            id_generator:InstanceIDGenerator {
                highest_id:Arc::new(AtomicUsize::new(highest_id)),
                dead_queue:channel(),
            }
        }
    }
}

#[derive(Clone)]
pub struct InstanceIDGenerator {
    pub highest_id:Arc<AtomicUsize>,
    pub dead_queue:(Sender<usize>, Receiver<usize>),
}

impl InstanceIDGenerator {
    pub fn get_next_id(&self) -> usize {
        match self.dead_queue.1.try_recv() {
            Ok(id) => id,
            Err(_) => self.highest_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        }
    }
    pub fn free_id(&self, id:usize) {
        self.dead_queue.0.send(id).unwrap();
    }
}

pub type MagicBuffer<T> = Vec<T>;

/// The vertex type that we will be used to describe the triangle's geometry.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub struct TriangleVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32 ; 2]
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32B32_SFLOAT)]
    pub world_position: [f32; 3],
    #[format(R32_SFLOAT)]
    pub scale: f32,
}

pub type TextureID = usize;

#[derive(Clone)]
pub struct MeshLOD {
    pub vertex_allocation:Allocation,
    pub index_allocation : Allocation,
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub struct IndexData {
    #[format(R32_UINT)]
    pub vertex:u32,
}

#[derive(Clone, ToBytes, FromBytes, PartialEq, Eq, Debug)]
pub enum MeshID {
    Name(String),
    DirectID(usize)
}

impl MeshID {
    pub fn unwrap_name(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::DirectID(_) => panic!("Unwrapped name on directID value")
        }
    }
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct CameraData {
    #[format(R32G32B32A32_SFLOAT)]
    view_0: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    view_1: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    view_2: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    view_3: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    projection_0: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    projection_1: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    projection_2: [f32 ; 4],
    #[format(R32G32B32A32_SFLOAT)]
    projection_3: [f32 ; 4],
}

impl CameraData {
    pub fn from_view_projection(view:Mat4, projection:Mat4) -> Self {
        Self {
            view_0:view.to_cols_array_2d()[0],
            view_1:view.to_cols_array_2d()[1],
            view_2:view.to_cols_array_2d()[2],
            view_3:view.to_cols_array_2d()[3],

            projection_0:projection.to_cols_array_2d()[0],
            projection_1:projection.to_cols_array_2d()[1],
            projection_2:projection.to_cols_array_2d()[2],
            projection_3:projection.to_cols_array_2d()[3],
        }
    }
}