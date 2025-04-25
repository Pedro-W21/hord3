use std::{collections::{HashMap, VecDeque}, ops::{Add, Range, Sub}, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use crate::horde::{geometry::{rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}, HordeFloat}, utils::parallel_counter::ParallelCounter};

use super::{rendering_spaces::{Vec3DfCam, Vec3DfRaster, ViewportData}, triangles::{SingleFullTriangle, TrianglePointData}};

#[derive(Clone, Debug)]
pub enum MeshID {
    Referenced(usize),
    Named(MeshName)
}

type MeshName = String;

#[derive(Clone)]
pub struct Meshes {
    all_meshes:Arc<RwLock<MeshesVec>>,
    reference_table:Arc<RwLock<HashMap<MeshName, usize>>>,
    instances:Arc<RwLock<Vec<MeshInstances>>>
}

pub struct MeshesVec {
    data:Vec<Mesh>,
    available:VecDeque<usize>,
}

impl MeshesVec {
    pub fn new(capacity:usize) -> Self {
        Self { data: Vec::with_capacity(capacity), available: VecDeque::with_capacity(capacity) }
    }
    pub fn change_lods_size_of(&mut self, for_mesh:usize, new_lods:MeshLODS, new_size:f32) {
        self.data[for_mesh].lods = new_lods;
        self.data[for_mesh].size = new_size;
    }
}

pub struct MeshInstances {
    available:VecDeque<usize>,
    instances:Vec<MeshInstance>,
    counter:ParallelCounter,
}

impl MeshInstances {
    pub fn with_capacity(capacity:usize, batch_size:usize) -> Self {
        Self { available: VecDeque::with_capacity(capacity), instances: Vec::with_capacity(capacity), counter: ParallelCounter::new(0, batch_size) }
    }
    pub fn instances_len(&self) -> usize {
        self.instances.len()
    }
    pub fn get_ready_counter(&self) -> ParallelCounter {
        let mut counter = self.counter.clone();
        counter.update_len(self.instances.len());
        counter.initialise();
        counter
    }
    pub fn get_instance(&self, index:usize) -> &MeshInstance {
        &self.instances[index]
    }
    pub fn get_instance_mut(&mut self, index:usize) -> &mut MeshInstance {
        &mut self.instances[index]
    }
}

pub struct MeshesRead<'a> {
    all_meshes:RwLockReadGuard<'a, MeshesVec>,
    reference_table:RwLockReadGuard<'a, HashMap<MeshName, usize>>,
    instances:RwLockReadGuard<'a, Vec<MeshInstances>>
}

impl<'a> MeshesRead<'a> {
    pub fn get_index_id(&self, id:&MeshID) -> usize {
        //dbg!(id);
        match id {
            MeshID::Referenced(index) => *index,
            MeshID::Named(name) => *self.reference_table.get(name).expect("Named mesh hasn't been loaded yet !!!!!!!")
        }
    }
    pub fn get_instances(&self) -> &Vec<MeshInstances> {
        &self.instances
    }
}

pub struct MeshesWrite<'a> {
    pub all_meshes:RwLockWriteGuard<'a, MeshesVec>,
    pub reference_table:RwLockWriteGuard<'a, HashMap<MeshName, usize>>,
    pub instances:RwLockWriteGuard<'a, Vec<MeshInstances>>
}

impl Meshes {
    pub fn get_read<'a>(&'a self) -> MeshesRead {
        MeshesRead { all_meshes:self.all_meshes.read().unwrap(), reference_table:self.reference_table.read().unwrap(), instances:self.instances.read().unwrap()  }
    }
    pub fn get_write<'a>(&'a self) -> MeshesWrite {
        MeshesWrite { all_meshes:self.all_meshes.write().unwrap(), reference_table:self.reference_table.write().unwrap(), instances:self.instances.write().unwrap() }
    }
    pub fn new(different_meshes_capacity:usize, instance_vecs_capacity:usize) -> Self {

        Self { 
            all_meshes: Arc::new(RwLock::new(MeshesVec::new(different_meshes_capacity))),
            reference_table: Arc::new(RwLock::new(HashMap::with_capacity(different_meshes_capacity))),
            instances: Arc::new(RwLock::new(Vec::with_capacity(instance_vecs_capacity)))
        }
    }
}

impl<'a> MeshesWrite<'a> {
    pub fn add_instance(&mut self, mut instance:MeshInstance, for_vec:usize) -> usize {
        let final_id = MeshID::Referenced(match &instance.mesh_id {
            MeshID::Referenced(refe) => *refe,
            MeshID::Named(name) => self.get_index_id(&instance.mesh_id)
        });
        instance.mesh_id = final_id;
        self.push_instance(instance, for_vec)
    }
    fn push_instance(&mut self, instance:MeshInstance, for_vec:usize) -> usize {
        if for_vec >= self.instances.len() {
            for i in self.instances.len()..(for_vec + 1) {
                self.instances.push(MeshInstances::with_capacity(256, 64));
            }
            self.push_instance(instance, for_vec)
        }
        else {
            match self.instances[for_vec].available.pop_back() {
                Some(index) => {self.instances[for_vec].instances[index] = instance; index},
                None => {let index = self.instances[for_vec].instances.len(); self.instances[for_vec].instances.push(instance); index }
            }
        }
    }
    pub fn change_buffer_size_for_instance_vec(&mut self, for_vec:usize, buffer_size:usize) {
        let mut real_buffer = buffer_size.max(1);
        if for_vec < self.instances.len() {
            self.instances[for_vec].counter.update_buffer_size(real_buffer);
        }
    }
    pub fn get_index_id(&self, id:&MeshID) -> usize {
        match id {
            MeshID::Referenced(index) => *index,
            MeshID::Named(name) => *self.reference_table.get(name).unwrap()
        }
    }
    pub fn does_mesh_exist(&self, id:&MeshID) -> bool {
        match id {
            MeshID::Referenced(index) => self.all_meshes.data.len() > *index,
            MeshID::Named(name) => self.reference_table.get(name).is_some()
        }
    }
    pub fn add_mesh(&mut self, mesh:Mesh) -> usize {
        let name = mesh.name.clone();
        match self.all_meshes.available.pop_back() {
            Some(index) => match self.reference_table.insert(name, index) {
                Some(previous_index) => {println!("This mesh name has already been loaded"); previous_index},
                None => {self.all_meshes.data[index] = mesh; index},
            },
            None => {
                let index = self.all_meshes.data.len();
                
                match self.reference_table.insert(name, index) {
                    Some(previous_index) => {println!("This mesh name has already been loaded"); previous_index},
                    None => {self.all_meshes.data.push(mesh); index},
                }
            }
        }
    }
    pub fn remove_mesh(&mut self, id:&MeshID) {
        match id {
            MeshID::Referenced(refe) => {
                self.all_meshes.available.push_back(*refe);
                let mesh = &self.all_meshes.data[*refe];
                self.reference_table.remove(&mesh.name);
            }
            MeshID::Named(name) => {
                let refe = self.reference_table.get(name).unwrap();
                self.all_meshes.available.push_back(*refe);
                self.reference_table.remove(name);
            }
        }
    }
    pub fn set_mesh(&mut self, id:&MeshID, new_val:Mesh) {
        let id = self.get_index_id(id);
        self.all_meshes.data[id] = new_val;
    }
    pub fn change_visibility_of_all_instances_of_vec(&mut self, vec:usize, visibility:bool) {
        if self.instances.len() > vec {
            for instance in &mut self.instances[vec].instances {
                instance.visible = visibility;
            }
        }
    }
    pub fn set_or_add_instance(&mut self, instance:MeshInstance, for_vec:usize, index:usize) {
        if for_vec >= self.instances.len() {
            for i in self.instances.len()..(for_vec + 1) {
                self.instances.push(MeshInstances::with_capacity(256, 64));
            }
        }
        if self.instances[for_vec].instances_len() <= index {
            for i in self.instances[for_vec].instances_len()..index {
                self.instances[for_vec].available.push_back(i);
                self.instances[for_vec].instances.push(MeshInstance::new(Vec3D::zero(), Orientation::zero(), MeshID::Referenced(0), false, true, false));
            }
            self.instances[for_vec].instances.push(instance);
        }
        else {
            self.instances[for_vec].instances[index] = instance;
        }
    }
    pub fn set_or_add_mesh(&mut self, id:&MeshID, new_val:Mesh) {
        match id {
            MeshID::Named(name) => match self.reference_table.get(name) {
                Some(mesh) => self.set_mesh(id, new_val),
                None => {self.add_mesh(new_val);},
            },
            MeshID::Referenced(refe) => if *refe < self.all_meshes.data.len() {
                self.set_mesh(id, new_val);
            }
            else {
                self.add_mesh(new_val);
            }
        }
    }
    pub fn remove_instance(&mut self, instance:usize, for_vec:usize) {
        self.instances[for_vec].instances[instance].visible = false;
        self.instances[for_vec].available.push_back(instance);
    }
}


pub struct MeshLOD {
    pub x:Vec<HordeFloat>,
    pub y:Vec<HordeFloat>,
    pub z:Vec<HordeFloat>,
    pub triangles:MeshTriangles,
}

#[derive(Clone)]
pub enum MeshLODType {
    Mesh(Arc<MeshLOD>),
    Image{texture:u32, collux_simple:(u8,u8,u8)}
}

impl MeshLOD {
    pub fn new(x:Vec<HordeFloat>, y:Vec<HordeFloat>, z:Vec<HordeFloat>, triangles:MeshTriangles) -> MeshLOD {
        MeshLOD { x, y, z, triangles }
    }
    pub fn add_point(&mut self, point:Vec3Df) {
        self.x.push(point.x);
        self.y.push(point.y);
        self.z.push(point.z);
    }
    pub fn add_points(&mut self, points:&[Vec3Df]) {
        for point in points {
            self.add_point(*point);
        }
    }
    pub fn merge_with(&mut self, other:MeshLOD) {
        let pre_len = self.x.len();
        for i in 0..other.x.len() {
            self.add_point(Vec3Df::new(other.x[i], other.y[i], other.z[i]));
        }
        for i in 0..other.triangles.len() {
            let (t1, t2, t3, texture, flag) = other.triangles.get_triangle_points_shifted_indices(i, pre_len);
            self.triangles.add_triangle(
                t1,
                t2,
                t3,
                texture, flag
            );
        }
    }
}

#[derive(Clone)]
pub struct MeshLODS {
    lods:Vec<MeshLODType>
}

impl MeshLODS {
    pub fn new(lods:Vec<MeshLODType>) -> Self {
        Self { lods }
    }
    pub fn get_lod(&self, lod:usize) -> MeshLODType {
        self.lods[lod].clone()
    }
    pub fn get_lod_bounded(&self, lod:usize) -> MeshLODType {
        self.lods[lod.clamp(0, self.lods.len() - 1)].clone()
    }
    pub fn set_lod(&mut self, lod:usize, new_lod:MeshLODType) {
        self.lods[lod] = new_lod;
    }
    pub fn new_lod(&mut self, new_lod:MeshLODType) {
        self.lods.push(new_lod)
    }
}


#[derive(Clone)]
pub struct Mesh {
    lods:MeshLODS,
    name:MeshName,
    size:HordeFloat,
}

impl Mesh {
    pub fn new(lods:MeshLODS, name:MeshName, size:HordeFloat) -> Self {
        Self { lods, name, size }
    }
    pub fn might_be_renderable(&self, poscam:&Vec3Df, rotat: &Rotation, at:&Vec3Df, viewport_data:&ViewportData) -> Option<Vec3DfCam> {
        let cam = Vec3DfCam::from_realspace(*at, poscam, rotat);
        let dist = viewport_data.camera_plane.signed_distance(&cam.0);
        if dist > 0.0 || dist.abs() < self.size  {
            Some(cam)
        }
        else {
            None
        } 
    }
    pub fn should_be_rendered(&self, camera_space:Vec3DfCam, viewport_data:&ViewportData) -> Option<f32> {
        let raster = Vec3DfRaster::from_cameraspace(camera_space, viewport_data);
        
        if raster.is_point_on_screen(viewport_data) {
            let r2 = (viewport_data.near_clipping_plane * viewport_data.image_width * raster.z.abs()) * self.size;
            Some(r2)
        }
        else {
            let r2 = (viewport_data.near_clipping_plane * viewport_data.image_width * raster.z.abs()) * self.size;
            if (raster.x < 0.0 && raster.x + r2 > 0.0) || (raster.y < 0.0 && raster.y + r2 > 0.0) || (raster.y > viewport_data.image_height && raster.y - r2 < viewport_data.image_height) || (raster.x > viewport_data.image_width && raster.x - r2 < viewport_data.image_width) {
                Some(r2)
            }
            else {
                None
            }
        }
    }
    pub fn get_rect_far(&self,camera_space: &Vec3DfCam,rotat: &Rotation, viewport_data:&ViewportData) -> Option<(Rectangle<i32>, f32)> {
        let orig_projec = Vec3DfRaster::from_cameraspace(camera_space.clone(), viewport_data);
        let points = [

            orig_projec,
            Vec3DfRaster(orig_projec.0
                + Vec3Df::new(
                    -self.size * 0.5 * orig_projec.z * viewport_data.half_image_width,
                    self.size * 0.5 * orig_projec.z * viewport_data.half_image_height * viewport_data.aspect_ratio,
                    -0.0001,
                )),
            Vec3DfRaster(orig_projec.0
                + Vec3Df::new(
                    -self.size * 0.5 * orig_projec.z * viewport_data.half_image_width,
                    -self.size * 0.5 * orig_projec.z * viewport_data.half_image_height * viewport_data.aspect_ratio,
                    -0.001,
                )),
            Vec3DfRaster(
                orig_projec.0
                    + Vec3Df::new(
                        self.size * 0.5 * orig_projec.z * viewport_data.half_image_width,
                        -self.size * 0.5 * orig_projec.z * viewport_data.half_image_height * viewport_data.aspect_ratio,
                        -0.0001,
                    ),
            ),
            Vec3DfRaster(
                orig_projec.0
                    - Vec3Df::new(
                        -self.size * 0.5 * orig_projec.z * viewport_data.half_image_width,
                        -self.size * 0.5 * orig_projec.z * viewport_data.half_image_height * viewport_data.aspect_ratio,
                        -0.001,
                    ),
            ),
        ];
        let mut dedans = false;
        for point in &points {
            dedans = dedans || point.is_point_on_screen(&viewport_data);
        }
        if dedans {
            Some(
                (
                    Rectangle::new(
                    points[2].0.x as i32,
                    points[2].0.y as i32,
                    points[4].0.x as i32,
                    points[4].0.y as i32,
                    ),
                    points[0].0.z
                )
            )
        }
        else {
            None
        }

    }
}

pub struct MeshTriangles {
    p1:TrianglePoints,
    p2:TrianglePoints,
    p3:TrianglePoints,
    texture_flags:Vec<(u32, u32)>,
}

impl MeshTriangles {
    pub fn with_capacity(capacity:usize) -> Self {
        Self { p1: TrianglePoints::with_capacity(capacity), p2: TrianglePoints::with_capacity(capacity), p3: TrianglePoints::with_capacity(capacity), texture_flags:Vec::with_capacity(capacity) }
    }
    pub fn reserve(&mut self, additional:usize) {
        self.texture_flags.reserve(additional);
        self.p1.reserve(additional);
        self.p2.reserve(additional);
        self.p3.reserve(additional);
    }
    pub fn clear(&mut self) {
        self.p1.clear();
        self.p2.clear();
        self.p3.clear();
        self.texture_flags.clear();
    }
    pub fn add_triangle(&mut self, p1:TrianglePoint, p2:TrianglePoint, p3:TrianglePoint, texture:u32, flags:u32) {
        self.p1.add_point(p1);
        self.p2.add_point(p2);
        self.p3.add_point(p3);
        self.texture_flags.push((texture, flags))
    }
    pub fn get_triangle(&self, x:&Vec<HordeFloat>, y:&Vec<HordeFloat>, z:&Vec<HordeFloat>, index:usize) -> SingleFullTriangle {
        let p1d = self.p1.get_point(x, y, z, index);
        let p2d = self.p2.get_point(x, y, z, index); 
        let p3d= self.p3.get_point(x, y, z, index);
        SingleFullTriangle::new(p1d, p2d, p3d, self.texture_flags[index])
    }
    pub fn get_just_pos_tri(&self, x:&Vec<HordeFloat>, y:&Vec<HordeFloat>, z:&Vec<HordeFloat>, index:usize) -> SingleFullTriangle {
        let p1d = self.p1.get_pos_point(x, y, z, index);
        let p2d = self.p2.get_pos_point(x, y, z, index); 
        let p3d= self.p3.get_pos_point(x, y, z, index);
        SingleFullTriangle::new(p1d, p2d, p3d, (0, 0))
    }
    pub fn get_triangle_points_shifted_indices(&self, at:usize, offset:usize) -> (TrianglePoint, TrianglePoint, TrianglePoint, u32, u32) {
        let mut first = self.p1.get_point_no_data(at);
        first.index += offset;
        let mut second = self.p2.get_point_no_data(at);
        second.index += offset;
        let mut third = self.p3.get_point_no_data(at);
        third.index += offset;
        (first, second, third, self.texture_flags[at].0, self.texture_flags[at].1)
        
    }
    pub fn get_indices_for_triangle(&self, index:usize) -> [usize ; 3] {
        [
            self.p1.index[index],
            self.p2.index[index],
            self.p3.index[index]
        ]
    }
    pub fn len(&self) -> usize {
        self.texture_flags.len()
    }
    pub fn get_triangle_uvs(&mut self, index:usize) -> ((usize,&mut UVRGBData), (usize,&mut UVRGBData), (usize,&mut UVRGBData)) {
        (
            (   
                self.p1.index[index],
                &mut self.p1.uv_rgb[index],
            ),
            (
                self.p2.index[index],
                &mut self.p2.uv_rgb[index],
            ),
            (
                self.p3.index[index],
                &mut self.p3.uv_rgb[index]
            )
        )
    }
}

pub struct TrianglePoint {
    pub index:usize,
    pub u:HordeFloat,
    pub v:HordeFloat,
    pub r:u8,
    pub g:u8,
    pub b:u8
}

impl TrianglePoint {
    pub fn new(index:usize, u:HordeFloat, v:HordeFloat, r:u8, g:u8, b:u8) -> Self {
        Self { index, u, v, r, g, b }
    }
}

pub struct TrianglePoints {
    index:Vec<usize>,
    uv_rgb:Vec<UVRGBData>
}

#[derive(Clone, Copy)]
pub struct UVRGBData {
    pub u:HordeFloat,
    pub v:HordeFloat,
    r:u8,
    g:u8,
    b:u8
}

impl TrianglePoints {
    pub fn with_capacity(capacity:usize) -> Self {
        Self {
            index: Vec::with_capacity(capacity),
            uv_rgb:Vec::with_capacity(capacity)
        }
    }
    pub fn reserve(&mut self, additional:usize) {
        self.index.reserve(additional);
        self.uv_rgb.reserve(additional);
    }
    pub fn clear(&mut self) {
        self.index.clear();
        self.uv_rgb.clear();
    }
    pub fn add_point(&mut self, point:TrianglePoint) -> usize {
        let index = self.index.len();
        self.index.push(point.index);
        self.uv_rgb.push(UVRGBData { u: point.u, v: point.v, r: point.r, g: point.g, b: point.b });
        index
    }
    pub fn get_point(&self, x:&Vec<f32>, y:&Vec<f32>, z:&Vec<f32>, index:usize) -> TrianglePointData {
        let pos_index = self.index[index];
        let uv_rgb = self.uv_rgb[index];
        TrianglePointData::new(Vec3DfCam(Vec3Df::new(x[pos_index], y[pos_index], z[pos_index])), uv_rgb.u, uv_rgb.v, uv_rgb.r, uv_rgb.g, uv_rgb.b)
    }
    pub fn get_pos_point(&self, x:&Vec<f32>, y:&Vec<f32>, z:&Vec<f32>, index:usize) -> TrianglePointData {
        let pos_index = self.index[index];
        TrianglePointData::new(Vec3DfCam(Vec3Df::new(x[pos_index], y[pos_index], z[pos_index])), 0.0, 0.0, 0, 0, 0)
    }
    pub fn get_point_no_data(&self, index:usize) -> TrianglePoint {

        let uv_rgb = self.uv_rgb[index];
        TrianglePoint::new(self.index[index], uv_rgb.u, uv_rgb.v, uv_rgb.r, uv_rgb.g, uv_rgb.b)
    }
}

pub struct MeshInstance {
    pos:Vec3Df,
    orient:Orientation,
    mesh_id:MeshID,
    visible:bool,
    worldpos_mesh:bool,
    viewmodel:bool,
}

pub struct InstanceRenderData<'a> {
    pub mesh:Arc<MeshLOD>,
    pub unique_data: &'a MeshInstance,
    pub rotation:Rotation
}

pub struct FlatInstanceRenderData<'a> {
    pub texture:usize,
    pub unique_data:&'a MeshInstance,
    pub rect_far:Rectangle<i32>,
    pub depth:f32,
    pub collux_simple:(u8,u8,u8)
}

pub enum EitherOrNone<T, U> {
    First(T),
    Second(U),
    None
}

impl MeshInstance {
    pub fn get_pos(&self) -> &Vec3Df {
        &self.pos
    }

    pub fn new(pos:Vec3Df, orient:Orientation, mesh_id:MeshID, visible:bool, worldpos_mesh:bool, viewmodel:bool) -> Self {
        Self { pos, orient, mesh_id, visible, worldpos_mesh, viewmodel }
    }

    pub fn change_pos(&mut self, new_pos:Vec3Df) {
        self.pos = new_pos;
    }

    pub fn change_visibility(&mut self, vis:bool) {
        self.visible = vis;
    }

    pub fn change_orient(&mut self, new_orient:Orientation) {
        self.orient = new_orient;
    }

    pub fn get_render_data_if_renderable<'a>(&'a self, meshes:&'a MeshesRead<'a>, poscam:&Vec3Df, rotat_cam:&Rotation, viewport_data:&ViewportData) -> EitherOrNone<InstanceRenderData<'a>, FlatInstanceRenderData<'a>> {
        //dbg!(self.mesh_id.clone());
        if self.visible {
            let mesh = &meshes.all_meshes.data[meshes.get_index_id(&self.mesh_id)];
            if self.viewmodel {
                let lod = mesh.lods.get_lod_bounded(0 );
                match lod {
                    MeshLODType::Mesh(mesh) => {
                        
                        EitherOrNone::First(InstanceRenderData { mesh, unique_data: self, rotation:Rotation::from_orientation(self.orient) })       
                    },
                    MeshLODType::Image { texture, collux_simple } => {
                        match mesh.get_rect_far(&Vec3DfCam(self.pos), rotat_cam, viewport_data) {
                            Some((rect_far, depth)) => EitherOrNone::Second(FlatInstanceRenderData { collux_simple, texture: texture as usize, unique_data: self, rect_far, depth }),
                            None => EitherOrNone::None
                        }
                    }
                }
            }
            else {  
                
                match mesh.might_be_renderable(poscam, rotat_cam, &self.pos, viewport_data) {
                    Some(camera_space) => match mesh.should_be_rendered(camera_space, viewport_data) {
                        Some(raster) => {
                            let lod = mesh.lods.get_lod_bounded((raster / 100.0) as usize );
                            match lod {
                                MeshLODType::Mesh(mesh) => {
                                    
                                    EitherOrNone::First(InstanceRenderData { mesh, unique_data: self, rotation:Rotation::from_orientation(self.orient) })       
                                },
                                MeshLODType::Image { texture, collux_simple } => {
                                    match mesh.get_rect_far(&camera_space, rotat_cam, viewport_data) {
                                        Some((rect_far, depth)) => EitherOrNone::Second(FlatInstanceRenderData { collux_simple, texture: texture as usize, unique_data: self, rect_far, depth }),
                                        None => EitherOrNone::None
                                    }
                                }
                            }   
                        },
                        _ => EitherOrNone::None
                    },
                    None => EitherOrNone::None
                }
            }
            
        }
        else {
            EitherOrNone::None
        }
        
    }

    pub fn is_worldpos(&self) -> bool {
        self.worldpos_mesh
    }
    pub fn is_viewmodel(&self) -> bool {
        self.viewmodel
    }
}


#[derive(Clone, Copy)]
pub struct Rectangle<T: PartialOrd + Clone + Copy + Sub<Output = T>> {
    x1: T,
    y1: T,
    x2: T,
    y2: T,
}

impl<T: Ord + Clone + Copy + Add<Output = T> + Sub<Output = T>> Rectangle<T> {
    pub fn new(x1: T, y1: T, x2: T, y2: T) -> Self {
        Self {
            x1: if x1 > x2 { x2 } else { x1 },
            y1: if y1 > y2 { y2 } else { y1 },
            x2: if x2 > x1 { x2 } else { x1 },
            y2: if y2 > y1 { y2 } else { y1 },
        }
    }
    pub fn add_to_outer_edges(self, val:T) -> Self {
        Self { x1: self.x1, y1: self.y1, x2: self.x2 + val, y2: self.y2 + val }
    }
    pub fn add_to_inner_edges(self, val:T) -> Self {
        Self { x1: self.x1 + val, y1: self.y1 + val, x2: self.x2, y2: self.y2 }
    }
    pub fn point_inside(&self, x:T, y:T) -> bool {
        x > self.x1 && x < self.x2 && y > self.y1 && y < self.y2
    }
    pub fn h_range(&self) -> Range<T> {
        self.x1..self.x2
    }
    pub fn v_range(&self) -> Range<T> {
        self.y1..self.y2
    }
    pub fn width(&self) -> T {
        self.x2 - self.x1
    }
    pub fn height(&self) -> T {
        self.y2 - self.y1
    }
    pub fn origin(&self) -> (T,T) {
        (self.x1, self.y1)
    }
    pub fn clip(self, min_x:T, min_y:T, max_x:T, max_y:T) -> Self {
        Self {
            x1: self.x1.max(min_x).min(max_x),
            y1: self.y1.max(min_y).min(max_y),
            x2: self.x2.max(min_x).min(max_x),
            y2: self.y2.max(min_y).min(max_y)
        }
    }
    pub fn any_range_empty(&self) -> bool {
        self.h_range().is_empty() || self.v_range().is_empty()
    }
}