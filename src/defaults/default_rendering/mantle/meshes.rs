use std::sync::{Arc, atomic::{AtomicU64, AtomicUsize}, mpmc::{Receiver, Sender, channel}};

use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, format, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::graphics::vertex_input::Vertex};

use crate::{defaults::default_rendering::mantle::api::{CPUInstanceData, MantleEvent, MantleRequest, MantleResponse}, horde::geometry::{rotation::Rotation, vec3d::Vec3Df}};


pub type MemoryAllocator = Arc<vulkano::memory::allocator::GenericMemoryAllocator<vulkano::memory::allocator::FreeListAllocator>>;
pub struct Meshes {
    pub meshes:Vec<Mesh>,
    pub mesh_creation_sender:Sender<MantleResponse>,
    pub allocator:MemoryAllocator,
}

impl Meshes {
    pub fn apply_event(&mut self, event:MantleEvent) {
        match event.update {
            MantleRequest::SetGlobalLOD { mesh_id, lod } => {
                self.get_mesh_mut(mesh_id).and_then(|mesh| {mesh.chosen_lod = lod; Some(1_u8)});
                event.response.send(MantleResponse::Success).unwrap();
            },
            MantleRequest::CreateInstance { mesh_id, chosen_id, new_data } => {
                let allocator = self.allocator.clone();
                self.get_mesh_mut(mesh_id).and_then(|mesh| {
                    let reallocate = {
                        let mut instances = mesh.instances.instance_buffer.write().unwrap();
                        if instances.len() < chosen_id {
                            let data = instances.to_vec();
                            Some(data)
                        }
                        else {
                            instances.get_mut(chosen_id).and_then(|data| {*data = InstanceData { world_position: new_data.position.coords_to_array(), scale: 1.0}; Some(1_u8)});
                            None
                        }    
                    };
                    match reallocate {
                        Some(mut data) => {
                            for i in (data.len().max(1) - 1)..chosen_id {
                                data.push(InstanceData { world_position: new_data.position.coords_to_array(), scale: 0.0});
                            }
                            data.last_mut().unwrap().scale = 1.0;

                            let instance_buffer = Buffer::from_iter(
                                allocator,
                                BufferCreateInfo {
                                    usage: BufferUsage::VERTEX_BUFFER,
                                    ..Default::default()
                                },
                                AllocationCreateInfo {
                                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                                    ..Default::default()
                                },
                                data,
                            )
                            .unwrap();
                            mesh.instances.instance_buffer = instance_buffer;
                        },
                        None => ()
                    }
                    

                    Some(1_u8)
                });
            },
            MantleRequest::UpdateInstance { mesh_id, instance, new_data } => {
                self.get_mesh_mut(mesh_id).and_then(|mesh| {
                    let mut instances = mesh.instances.instance_buffer.write().unwrap();
                    instances.get_mut(instance).and_then(|data| {*data = InstanceData { world_position: new_data.position.coords_to_array(), scale: 1.0}; Some(1_u8)});
                    Some(1_u8)
                });
            },
            MantleRequest::RemoveInstance { mesh_id, removed_id } => (),
            MantleRequest::CreateOrUpdateMesh { name, lods, texture, first_instances } => {
                let meshlods:Vec<MeshLOD> = lods.into_iter().map(|apilod| {

                    let vertex_buffer = Buffer::from_iter(
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
                        apilod.vertex_data.iter().map(|vertex| {TriangleVertex {position:vertex.position.coords_to_array()}}),
                    )
                    .unwrap();

                    let index_buffer = Buffer::from_iter(
                        self.allocator.clone(),
                        BufferCreateInfo {
                            usage: BufferUsage::INDEX_BUFFER,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                            ..Default::default()
                        },
                        apilod.index_data.iter().map(|index| {index.vertex}),
                    )
                    .unwrap();
                    MeshLOD { vertex_buffer, indices: index_buffer, texture }
                }).collect();
                if let Some(mesh) = self.get_mesh_mut(MeshID::Name(name.clone())) {
                    mesh.lods = meshlods;
                    event.response.send(MantleResponse::Success).unwrap();
                }
                else {
                    let id = self.meshes.len();
                    let instances = Instances::new(first_instances, self.allocator.clone());
                    let id_generator = instances.id_generator.clone();
                    self.meshes.push(Mesh { name:name.clone(), lods:meshlods, instances, chosen_lod: None });
                    self.mesh_creation_sender.send(MantleResponse::MeshCreated { id_generator:id_generator.clone(), direct_id:id, name:name.clone() }).unwrap();
                    event.response.send(MantleResponse::MeshCreated { id_generator, direct_id:id, name }).unwrap();
                }
            }
        }
    }
    pub fn get_mesh_mut(&mut self, id:MeshID) -> Option<&mut Mesh> {
        match id {
            MeshID::DirectID(index) => self.meshes.get_mut(index),
            MeshID::Name(name) => self.meshes.iter_mut().find(|mesh| {mesh.name == name})
        }
    }
}

pub struct Mesh {
    pub name:String,
    pub lods:Vec<MeshLOD>,
    pub instances:Instances,
    pub chosen_lod:Option<usize>
}

pub struct Instances {
    pub instance_buffer: Subbuffer<[InstanceData]>,
    pub id_generator:InstanceIDGenerator
    
}

impl Instances {
    pub fn new(first_instances:Vec<CPUInstanceData>,allocator:MemoryAllocator) -> Self {
        let mut data = Vec::with_capacity(first_instances.len());
        let highest_id = first_instances.len();
        for instance in first_instances {
            data.push(InstanceData { world_position: instance.position.coords_to_array(), scale: 1.0});
        }
        let instance_buffer = Buffer::from_iter(
            allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )
        .unwrap();
        Self {
            instance_buffer,
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
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct TriangleVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32B32_SFLOAT)]
    pub world_position: [f32; 3],
    #[format(R32_SFLOAT)]
    pub scale: f32,
}

pub type TextureID = usize;
pub struct MeshLOD {

    pub vertex_buffer: Subbuffer<[TriangleVertex]>,
    pub indices:Subbuffer<[u32]>,
    pub texture: TextureID
}

/// The vertex type that describes the unique data per instance.
#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct IndexData {
    #[format(R32_UINT)]
    pub vertex:u32,
    #[format(R32G32_SFLOAT)]
    pub u:[f32 ; 2],
}

pub enum MeshID {
    Name(String),
    DirectID(usize)
}