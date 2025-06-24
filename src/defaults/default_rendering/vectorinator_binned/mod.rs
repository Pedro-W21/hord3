use std::{cell::SyncUnsafeCell, f32::INFINITY, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}, thread, time::Duration};

use bins::{Bins, PreCalcdData};
use meshes::{EitherOrNone, MeshID, Meshes, MeshesRead, MeshesWrite};
use rasterisation::{rasterise_if_possible, textured_rect_at_depth, InternalRasterisationData};
use rendering_spaces::ViewportData;
use shaders::{ShaderData, ShaderFrameData};
use textures::{rgb_to_argb, Textures};
use threading_utils::utils::step_sync::StepSync;
use triangles::{LessAllocTransformedMesh, SingleFullTriangle, TransformedMesh};

use crate::horde::{frontend::{HordeWindowDimensions, SyncUnsafeHordeFramebuffer}, geometry::{plane::EquationPlane, rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}}, rendering::{camera::Camera, framebuffer::HordeColorFormat, RenderingBackend}, scheduler::IndividualTask, utils::parallel_counter::ParallelCounter};

pub mod meshes;
pub mod rendering_spaces;
pub mod triangles;
pub mod simd_geo;
pub mod consts;
pub mod rasterisation;
pub mod textures;
pub mod shapes_to_tris;
pub mod bins;
pub mod shaders;

#[derive(Clone)]
pub struct Vectorinator<SD:ShaderData> {
    pub framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>,
    pub internal_framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>,
    pub zbuf:Arc<RwLock<SyncUnsafeCell<Vec<f32>>>>,
    pub nbuf:Arc<RwLock<SyncUnsafeCell<Vec<u32>>>>,
    pub meshes:Meshes,
    pub textures:Arc<RwLock<Textures>>,
    pub latest_camera:Arc<RwLock<Camera>>,
    pub empty_zbuf:Arc<Vec<f32>>,
    pub empty_nbuf:Arc<Vec<u32>>,
    pub color_vec:Arc<RwLock<Vec<u32>>>,
    pub bins:Arc<Bins>,
    pub shader_data:Arc<SD>,
    step_sync:StepSync,
    shader_step_sync:StepSync,
    shader_counter:ParallelCounter
}

pub struct VectorinatorRead<'a, SD:ShaderData> {
    framebuf:RwLockReadGuard<'a, SyncUnsafeHordeFramebuffer>,
    internal_framebuf:RwLockReadGuard<'a, SyncUnsafeHordeFramebuffer>,
    zbuf:RwLockReadGuard<'a, SyncUnsafeCell<Vec<f32>>>,
    nbuf:RwLockReadGuard<'a, SyncUnsafeCell<Vec<u32>>>,
    meshes:MeshesRead<'a>,
    pub textures:RwLockReadGuard<'a, Textures>,
    camera:RwLockReadGuard<'a, Camera>,
    empty_zbuf:&'a Vec<f32>,
    empty_nbuf:&'a Vec<u32>,
    bins:Arc<Bins>,
    color_vec:RwLockReadGuard<'a, Vec<u32>>,
    shader_data:Arc<SD>,
    step_sync:StepSync,
    shader_step_sync:StepSync,
    shader_counter:ParallelCounter
}

impl<'a, SD:ShaderData> VectorinatorRead<'a, SD> {
    pub fn render_everything(&'a self, viewport_data:ViewportData, number_of_threads:usize) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        self.shader_counter.reset();
        self.shader_counter.update_len(viewport_data.image_height as usize * viewport_data.image_width as usize);
        self.bins.reset_internal_counter();
        //let mut total_tris = 0;
        self.step_sync.start_action(number_of_threads);
        let mut transformed = LessAllocTransformedMesh::new(256);
        for instances in self.meshes.get_instances() {
            //dbg!(instances.instances_len());
            let counter = instances.get_ready_counter();
            for instance in counter {
                let instance_data = instances.get_instance(instance);
                match instance_data.get_render_data_if_renderable(&self.meshes, &viewport_data.poscam, &viewport_data.rotat_cam, &viewport_data) {
                    EitherOrNone::First(render_data) => {
                        transformed.clear_with_new_capacity(render_data.mesh.x.len());
                        let tf_mesh = if instance_data.is_worldpos() {
                            transformed.from_realspace_worldpos(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam);
                            //TransformedMesh::from_realspace_worldpos(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam)
                        }
                        else if instance_data.is_viewmodel() {
                            transformed.from_realspace_viewmodel(&render_data);
                            // println!("Viewmodel");
                            //TransformedMesh::from_realspace_viewmodel(&render_data)
                        }
                        else {
                            transformed.from_realspace(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam);
                            //TransformedMesh::from_realspace(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam)
                        };
                        //let (mut final_mesh, mut final_tris) = tf_mesh.get_final_camera_triangles(&render_data.mesh.triangles, &viewport_data);
                        transformed.get_final_camera_triangles(&render_data.mesh.triangles, &viewport_data, &render_data);
                        transformed.cameraspace_to_rasterspace(&viewport_data);
                        //final_mesh.cameraspace_to_rasterspace(&viewport_data, &mut final_tris);
                        //dbg!(final_tris.len());
                        //total_tris += final_tris.len();
                        for tri in 0..transformed.mesh_tris.len() {
                            let triangle = transformed.mesh_tris.get_triangle(&transformed.new_x, &transformed.new_y, &transformed.new_z, tri);
                            //let triangle = final_tris.get_triangle(&final_mesh.x, &final_mesh.y, &final_mesh.z, tri);
                            let area = triangle.get_area();
                            let pre_bounding_box = triangle.pre_pre_bounding_box_f();
                            if area > 0.0 && (pre_bounding_box[0] < viewport_data.image_width && pre_bounding_box[1] < viewport_data.image_height) && (pre_bounding_box[2] > 0.0 && pre_bounding_box[3] > 0.0) {// && triangle.p1.is_point_on_screen(&viewport_data){
                                let mut pre_calcd = PreCalcdData::new(area, ((0, 0), (0, 0)), 0, transformed.normals_per_tri[tri]);
                                let mip_map = get_mip_map(&mut internal_data, &triangle, pre_calcd.inv_area);
                                pre_calcd.mip_map = mip_map;
                                self.bins.push_triangle(triangle, pre_calcd, triangle.pre_bounding_box_f(viewport_data.image_width, viewport_data.image_height, pre_bounding_box));
                            }
                        }
                    },
                    EitherOrNone::Second(render_data) => {
                        textured_rect_at_depth(render_data.rect_far, render_data.depth, render_data.texture as u16, &mut internal_data, render_data.collux_simple, &viewport_data);
                    },
                    EitherOrNone::None => ()
                }
            }
        }
        self.step_sync.wait_here(number_of_threads);
        self.shader_step_sync.start_action(number_of_threads);

        self.bins.do_for_all_triangle_ids(&mut |tri, bin| {
            // dbg!(tri.clone());
            //println!("WTF");
            rasterise_if_possible(tri, &internal_data, bin);
        }, &mut |bin| {
            let mut new_internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
            new_internal_data.copy_from_bin(bin);
            unsafe {bin.image_data.get().as_mut().unwrap_unchecked().clear_bufs(self.bins.get_clear());};
        });
        self.shader_step_sync.wait_here(number_of_threads);
        let mut counter = self.shader_counter.clone();
        counter.initialise();
        let mut shader_frame_data = self.shader_data.get_frame_data(&self.camera, &viewport_data.rotat_cam);
        unsafe {
            let old_framebuf = & *self.internal_framebuf.get_data_cell().get();
            let old_zbuf = & *self.zbuf.get();
            let old_nbuf = & * self.nbuf.get();
            let mut actual_framebuf = &mut *self.framebuf.get_data_cell().get();
            for i in counter {
                *actual_framebuf.get_unchecked_mut(i) = shader_frame_data.get_new_pixel(i, *old_framebuf.get_unchecked(i), *old_zbuf.get_unchecked(i), *old_nbuf.get_unchecked(i), old_framebuf, old_zbuf, old_nbuf, self.internal_framebuf.get_dims().get_width(),  self.internal_framebuf.get_dims().get_height());
            }
        }
        self.reset_counters();
        //println!("TOTAL TRIS THIS FRAME {}", total_tris);
    }
    fn clear_framebuf(&self, viewport_data:ViewportData) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        internal_data.clear_framebuf(&self.color_vec);
        unsafe {(&mut *self.framebuf.get_data_cell().get()).copy_from_slice(&self.color_vec);}
    }
    fn clear_zbuf(&self, viewport_data:ViewportData) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        internal_data.clear_zbuf(&self.empty_zbuf);
        internal_data.clear_nbuf(&self.empty_nbuf);
        self.bins.drop_triangles();
    }
    pub fn reset_counters(&self) {
        for instances in self.meshes.get_instances() {
            let counter = instances.get_ready_counter();
            counter.reset();
        }
    }
}

pub fn get_mip_map<'a>(data:&mut InternalRasterisationData<'a>, triangle:&SingleFullTriangle, inv_area:f32) -> usize {
    let texture = data.textures.get_text_with_id(triangle.texture_flags.0 as usize);
    let len = texture.get_mip_map(0).len_f32;
    let mip_map = ((len * inv_area).log2()) as usize;
    mip_map
}

impl<SD:ShaderData> Vectorinator<SD> {
    pub fn new(framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>, shader_data:Arc<SD>) -> Self {
        let (width, height) = (framebuf.read().unwrap().get_dims().get_width(), framebuf.read().unwrap().get_dims().get_height());
        Self {empty_nbuf:Arc::new(vec![0; width * height]),nbuf:Arc::new(RwLock::new(SyncUnsafeCell::new(vec![0; width * height]))), shader_counter:ParallelCounter::new(width * height, width),shader_data,shader_step_sync:StepSync::new(),step_sync:StepSync::new(),bins:Arc::new(Bins::new(framebuf.clone().read().unwrap().get_dims(), 20)), color_vec:Arc::new(RwLock::new(vec![0 ; width * height])), empty_zbuf:Arc::new(vec![INFINITY ; width * height]),internal_framebuf:Arc::new(RwLock::new(SyncUnsafeHordeFramebuffer::new(HordeWindowDimensions::new(width, height), HordeColorFormat::ARGB8888))),framebuf, zbuf:Arc::new(RwLock::new(SyncUnsafeCell::new(vec![0.0; width * height]))), meshes:Meshes::new(1000, 5), textures:Arc::new(RwLock::new(Textures::new())), latest_camera:Arc::new(RwLock::new(Camera::empty())) }
    }
    pub fn get_read<'a>(&'a self) -> VectorinatorRead<'a, SD> {
        VectorinatorRead {empty_nbuf:&self.empty_nbuf, nbuf:self.nbuf.read().unwrap(),internal_framebuf:self.internal_framebuf.read().unwrap(),shader_data:self.shader_data.clone(),shader_counter:self.shader_counter.clone(),shader_step_sync:self.shader_step_sync.clone(),step_sync:self.step_sync.clone(), bins:self.bins.clone(),color_vec:self.color_vec.read().unwrap(), empty_zbuf:&self.empty_zbuf, framebuf: self.framebuf.read().unwrap(), zbuf: self.zbuf.read().unwrap(), meshes: self.meshes.get_read(), textures: self.textures.read().unwrap(), camera:self.latest_camera.read().unwrap() }
    }
    pub fn get_texture_read<'a>(&'a self) -> RwLockReadGuard<'a, Textures> {
        self.textures.read().unwrap()
    }
    pub fn tick_all_sets(&self) {
        let mut textures_write = self.textures.write().unwrap();
        textures_write.tick_all_sets();
    }
    pub fn get_viewport_data(&self) -> ViewportData {
        let cam = self.latest_camera.read().unwrap();
        let framebuf = self.framebuf.read().unwrap();
        ViewportData {
            near_clipping_plane: 1.0,
            half_image_width: (framebuf.get_dims().get_width()/2) as f32,
            half_image_height: (framebuf.get_dims().get_height()/2) as f32,
            aspect_ratio: (framebuf.get_dims().get_width() as f32)/(framebuf.get_dims().get_height() as f32),
            camera_plane: EquationPlane::new(Vec3D::new(0.0, 0.0, 1.0), -1.0),
            image_height: (framebuf.get_dims().get_height() as f32),
            image_width: (framebuf.get_dims().get_width() as f32),
            poscam: cam.pos.clone(),
            rotat_cam: Rotation::new_from_inverted_orient(cam.orient.clone())
        }
    }
    pub fn get_write<'a>(&'a self) -> VectorinatorWrite<'a> {
        VectorinatorWrite { framebuf: self.framebuf.write().unwrap(), zbuf: self.zbuf.write().unwrap(), meshes: self.meshes.get_write(), textures: self.textures.write().unwrap(), camera:self.latest_camera.write().unwrap() }
    }
}

pub struct VectorinatorWrite<'a> {
    pub framebuf:RwLockWriteGuard<'a, SyncUnsafeHordeFramebuffer>,
    pub zbuf:RwLockWriteGuard<'a, SyncUnsafeCell<Vec<f32>>>,
    pub meshes:MeshesWrite<'a>,
    pub textures:RwLockWriteGuard<'a, Textures>,
    pub camera:RwLockWriteGuard<'a, Camera>
}

impl<SD:ShaderData> IndividualTask for Vectorinator<SD> {
    type TD = usize;
    type TID = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => { // Any number of threads
                
                let reader = self.get_read();
                let viewport = self.get_viewport_data();
                reader.render_everything(viewport, number_of_threads);
            },
            1 => { // Only one thread
                self.tick_all_sets();
            }
            2 => { // Any number of threads (though one is enough)
                self.get_read().reset_counters();
            },
            3 => { // Only one thread
                let reader = self.get_read();
                let viewport = self.get_viewport_data();
                reader.clear_framebuf(viewport);
            },
            4 => { // Only one thread
                let reader = self.get_read();
                let viewport = self.get_viewport_data();
                reader.clear_zbuf(viewport);
            },
            5 => { // Only one thread
                let reader = self.get_read();
                reader.framebuf.change_phase();
            },

            _ => panic!("NO TASK ID BIGGER THAN 4")
        }
    }
}
