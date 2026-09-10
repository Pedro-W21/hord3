use std::{collections::HashMap, sync::{Arc, RwLock, RwLockReadGuard, mpmc::{Receiver, Sender, channel}}};

use crate::{defaults::default_rendering::mantle::{meshes::{IndexData, InstanceIDGenerator, MeshID, TextureID}, textures::{buffer_to_request, load_single_texture}}, horde::{frontend::{MouseState, WindowingEvent}, geometry::{rotation::{Orientation, Rotation}, vec3d::Vec3Df}, rendering::camera::Camera, scheduler::IndividualTask}};


#[derive(Clone)]
pub struct CPUInstanceData {
    pub position:Vec3Df,
    pub speed:Vec3Df,
    pub rotation:Rotation,
}

#[derive(Clone)]
pub struct CPUVertexData {
    pub position:Vec3Df,
    pub texture_id:u8,
    pub u:f32,
    pub v:f32,
}

impl CPUVertexData {
    pub fn new(position:Vec3Df, texture_id:u8, u:f32, v:f32) -> Self {
        Self { position, texture_id, u, v }
    }
}

#[derive(Clone)]
pub struct ApiLod {
    pub textures:Vec<String>,
    pub vertex_data:Vec<CPUVertexData>,
    pub index_data:Vec<IndexData>
}

impl ApiLod {
    pub fn new(textures:Vec<String>, vertex_data:Vec<CPUVertexData>, index_data:Vec<IndexData>) -> Self {
        Self { textures, vertex_data, index_data }
    }
    pub fn new_simple(textures:Vec<String>, vertex_data:Vec<CPUVertexData>, index_data:Vec<u32>) -> Self {
        Self { textures, vertex_data, index_data:index_data.into_iter().map(|id| {IndexData {vertex:id}}).collect() }
    }
    pub fn add_points<const N:usize>(&mut self, points:[CPUVertexData ; N]) {
        for point in points {
            self.vertex_data.push(point);
        }
    }
    pub fn add_triangle(&mut self, p1:TrianglePoint, p2:TrianglePoint, p3:TrianglePoint) {
        self.index_data.push(IndexData { vertex: p1.index as u32 });
        self.index_data.push(IndexData { vertex: p2.index as u32 });
        self.index_data.push(IndexData { vertex: p3.index as u32 });
    }
    pub fn merge_with(&mut self, other:ApiLod) {
        let mut textures_lookup = HashMap::with_capacity(other.textures.len());
        for text in &other.textures {
            match self.textures.iter().enumerate().find(|(i, name)| {
                name == &text
            }) {
                Some(matched) => {
                    textures_lookup.insert(text.clone(), matched.0);
                },
                None => {
                    textures_lookup.insert(text.clone(), self.textures.len());
                    self.textures.push(text.clone());
                }
            }
        }
        let self_len = self.vertex_data.len();
        for v in other.vertex_data {
            self.vertex_data.push(CPUVertexData { position: v.position, texture_id: *textures_lookup.get(&other.textures[v.texture_id as usize]).unwrap() as u8, u: v.u, v: v.v });
        }
        for i in other.index_data {
            self.index_data.push(IndexData { vertex: i.vertex + self_len as u32 });
        }
    }
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
        first_instances:Vec<CPUInstanceData>
    },
    UpdateCamera {
        new_cam:Camera
    },
    CreateOrUpdateTexture {
        name:String,
        texture_data:Vec<u8>,
        width:usize,
        height:usize,
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
    mesh_datas:Arc<RwLock<Vec<CPUMeshData>>>,
    mouse_state:MouseState,
    outside_events:Receiver<WindowingEvent>
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
    pub fn new(event_sender:Sender<MantleEvent>, mesh_creation_receiver:Receiver<MantleResponse>, mouse_state:MouseState, outside_events:Receiver<WindowingEvent>) -> Self {
        Self { event_sender, mesh_creation_receiver, mesh_datas: Arc::new(RwLock::new(Vec::with_capacity(128))), mouse_state, outside_events }
    }
    pub fn get_meshes<'a>(&'a self) -> RwLockReadGuard<'a, Vec<CPUMeshData>> {
        self.mesh_datas.read().unwrap()
    }
    pub fn get_write(&self) -> MantleHandler {
        self.clone()
    }
    pub fn get_mouse_state(&self) -> MouseState {
        self.mouse_state.clone()
    }
    pub fn get_outside_events(&self) -> Receiver<WindowingEvent> {
        self.outside_events.clone()
    }
    pub fn set_or_add_mesh(&self, name:String, lods:Vec<ApiLod>, first_instances:Vec<CPUInstanceData>) -> Receiver<MantleResponse> {
        let (s,r) = channel();
        self.event_sender.send(MantleEvent {
            update: MantleRequest::CreateOrUpdateMesh { name, lods, first_instances },
            response: s
        }).unwrap();
        r
    }
    pub fn get_mesh(&self, mesh_id:MeshID) -> Option<CPUMeshData> {
        match mesh_id {
            MeshID::DirectID(i) => self.mesh_datas.read().unwrap().get(i).cloned(),
            MeshID::Name(name) => self.mesh_datas.read().unwrap().iter().find(|a| {a.name == name}).cloned(),
        }
    }
    pub fn add_instance(&self, mesh_id:MeshID, new_data:CPUInstanceData) -> (usize, Receiver<MantleResponse>) {
        let (s,r) = channel();
        let mesh_data = self.get_mesh(mesh_id.clone()).unwrap();
        let id = mesh_data.instance_id_generator.get_next_id();
        self.event_sender.send(MantleEvent {
            update: MantleRequest::CreateInstance { mesh_id, chosen_id: id, new_data },
            response: s
        }).unwrap();
        (id, r)
    }
    pub fn update_instance(&self, mesh_id:MeshID, instance:usize, new_data:CPUInstanceData) -> Receiver<MantleResponse> {

        let (s,r) = channel();
        self.event_sender.send(MantleEvent {
            update: MantleRequest::UpdateInstance { mesh_id, instance, new_data },
            response: s
        }).unwrap();
        r
    }
    pub fn update_camera(&self, new_cam:Camera) -> Receiver<MantleResponse> {

        let (s,r) = channel();
        self.event_sender.send(MantleEvent {
            update: MantleRequest::UpdateCamera { new_cam },
            response: s
        }).unwrap();
        r
    }
    pub fn create_or_update_texture(&self, texture_path:&str, texture_name:String) -> Receiver<MantleResponse> {
        let text = load_single_texture(texture_path).expect("Texture couldn't be loaded");
        let req = buffer_to_request(texture_name.clone(), text);

        let (s,r) = channel();
        self.event_sender.send(MantleEvent {
            update: req,
            response: s
        }).unwrap();
        r
    }
}

#[derive(Clone)]
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


pub struct TrianglePoint {
    pub index:usize,
    pub u:f32,
    pub v:f32,
    pub r:u8,
    pub g:u8,
    pub b:u8
}

impl TrianglePoint {
    pub fn new(index:usize, u:f32, v:f32, r:u8, g:u8, b:u8) -> Self {
        Self { index, u, v, r, g, b }
    }
}