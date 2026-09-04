use std::sync::{Arc, RwLock, RwLockReadGuard, mpmc::{Receiver, Sender}};

use crate::{defaults::default_rendering::mantle::meshes::{IndexData, InstanceIDGenerator, MeshID, TextureID}, horde::{geometry::{rotation::Rotation, vec3d::Vec3Df}, scheduler::IndividualTask}};


pub struct CPUInstanceData {
    pub position:Vec3Df,
    pub speed:Vec3Df,
    pub rotation:Rotation,
}

pub struct CPUVertexData {
    pub position:Vec3Df
}

pub struct ApiLod {
    pub vertex_data:Vec<CPUVertexData>,
    pub index_data:Vec<IndexData>
}

pub enum MantleRequest {
    UpdateInstance {
        mesh_id:MeshID,
        instance:usize,
        new_data:CPUInstanceData,
    },
    CreateInstance {
        mesh_id:MeshID,
        chosen_id:usize,
        new_data:CPUInstanceData,
    },
    RemoveInstance {
        mesh_id:MeshID,
        removed_id:usize,
    },
    SetGlobalLOD {
        mesh_id:MeshID,
        lod:Option<usize>
    },
    CreateOrUpdateMesh {
        name:String,
        lods:Vec<ApiLod>,
        texture:TextureID,
        first_instances:Vec<CPUInstanceData>
    }
}
pub struct MantleEvent {
    pub update:MantleRequest,
    pub response:Sender<MantleResponse>
}

pub enum MantleResponse {
    Success,
    Failure,
    MeshCreated {id_generator:InstanceIDGenerator, direct_id:usize, name:String}
}

#[derive(Clone)]
pub struct MantleHandler {
    pub event_sender:Sender<MantleEvent>,
    mesh_creation_receiver:Receiver<MantleResponse>,
    mesh_datas:Arc<RwLock<Vec<CPUMeshData>>>
}

impl MantleHandler {
    pub fn apply_creations(&self) {
        let mut datas = self.mesh_datas.write().unwrap();
        while let Ok(MantleResponse::MeshCreated { id_generator, direct_id, name }) = self.mesh_creation_receiver.try_recv() {
            if datas.len() > direct_id {
                datas[direct_id].instance_id_generator = id_generator;
                datas[direct_id].name = name;
            }
            else if datas.len() == direct_id {
                datas.push(CPUMeshData { instance_id_generator: id_generator, name });
            }
            else {
                for i in datas.len()..=direct_id {
                    datas.push(CPUMeshData { instance_id_generator: id_generator.clone(), name:name.clone() });
                }
            }
        }
    }
    pub fn new(event_sender:Sender<MantleEvent>, mesh_creation_receiver:Receiver<MantleResponse>) -> Self {
        Self { event_sender, mesh_creation_receiver, mesh_datas: Arc::new(RwLock::new(Vec::with_capacity(128))) }
    }
    pub fn get_meshes<'a>(&'a self) -> RwLockReadGuard<'a, Vec<CPUMeshData>> {
        self.mesh_datas.read().unwrap()
    }
}

pub struct CPUMeshData {
    instance_id_generator:InstanceIDGenerator,
    name:String,
}

impl IndividualTask for MantleHandler {
    type TID = usize;
    type TD = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => {
                self.apply_creations();
            },
            i => panic!("Task ID {i} not supported for this type")
        }
    }
}