use std::{cell::SyncUnsafeCell, f32::INFINITY, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use meshes::{EitherOrNone, MeshID, Meshes, MeshesRead, MeshesWrite};
use rasterisation::{rasterise_if_possible, textured_rect_at_depth, InternalRasterisationData};
use rendering_spaces::ViewportData;
use textures::{rgb_to_argb, Textures};
use triangles::TransformedMesh;

use crate::horde::{frontend::SyncUnsafeHordeFramebuffer, geometry::{plane::EquationPlane, rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}}, rendering::{camera::Camera, RenderingBackend}, scheduler::IndividualTask};

pub mod meshes;
pub mod rendering_spaces;
pub mod triangles;
pub mod simd_geo;
pub mod consts;
pub mod rasterisation;
pub mod textures;
pub mod shapes_to_tris;

#[derive(Clone)]
pub struct Vectorinator {
    pub framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>,
    pub zbuf:Arc<RwLock<SyncUnsafeCell<Vec<f32>>>>,
    pub meshes:Meshes,
    pub textures:Arc<RwLock<Textures>>,
    pub latest_camera:Arc<RwLock<Camera>>,
    pub empty_zbuf:Arc<Vec<f32>>,
    pub color_vec:Arc<RwLock<Vec<u32>>>,
}

pub struct VectorinatorRead<'a> {
    framebuf:RwLockReadGuard<'a, SyncUnsafeHordeFramebuffer>,
    zbuf:RwLockReadGuard<'a, SyncUnsafeCell<Vec<f32>>>,
    meshes:MeshesRead<'a>,
    textures:RwLockReadGuard<'a, Textures>,
    camera:RwLockReadGuard<'a, Camera>,
    empty_zbuf:&'a Vec<f32>,
    color_vec:RwLockReadGuard<'a, Vec<u32>>,
}

impl<'a> VectorinatorRead<'a> {
    pub fn render_everything(&'a self, viewport_data:ViewportData) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        //let mut total_tris = 0;
        for instances in self.meshes.get_instances() {
            //dbg!(instances.instances_len());
            let counter = instances.get_ready_counter();
            for instance in counter {
                let instance_data = instances.get_instance(instance);
                match instance_data.get_render_data_if_renderable(&self.meshes, &viewport_data.poscam, &viewport_data.rotat_cam, &viewport_data) {
                    EitherOrNone::First(render_data) => {
                        let tf_mesh = if instance_data.is_worldpos() {
                            TransformedMesh::from_realspace_worldpos(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam)
                        }
                        else if instance_data.is_viewmodel() {
                            println!("Viewmodel");
                            TransformedMesh::from_realspace_viewmodel(&render_data)
                        }
                        else {
                            TransformedMesh::from_realspace(&render_data, &viewport_data.poscam, &viewport_data.rotat_cam)
                        };
                        let (mut final_mesh,mut final_tris) = tf_mesh.get_final_camera_triangles(&render_data.mesh.triangles, &viewport_data);
                        final_mesh.cameraspace_to_rasterspace(&viewport_data, &mut final_tris);
                        //dbg!(final_tris.len());
                        //total_tris += final_tris.len();
                        for tri in 0..final_tris.len() {
                            let triangle = final_tris.get_triangle(&final_mesh.x, &final_mesh.y, &final_mesh.z, tri);
                            rasterise_if_possible(triangle, &mut internal_data);
                        }

                    },
                    EitherOrNone::Second(render_data) => {
                        textured_rect_at_depth(render_data.rect_far, render_data.depth, render_data.texture as u16, &mut internal_data, render_data.collux_simple, &viewport_data);
                    },
                    EitherOrNone::None => ()
                }
            }
        }
        //println!("TOTAL TRIS THIS FRAME {}", total_tris);
    }
    fn clear_framebuf(&self, viewport_data:ViewportData) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        internal_data.clear_framebuf(&self.color_vec);
    }
    fn clear_zbuf(&self, viewport_data:ViewportData) {
        let mut internal_data = InternalRasterisationData::from_vectorinator_read(self, viewport_data.clone());
        internal_data.clear_zbuf(&self.empty_zbuf);
    }
    pub fn reset_counters(&self) {
        for instances in self.meshes.get_instances() {
            let counter = instances.get_ready_counter();
            counter.reset();
        }
    }
}

impl Vectorinator {
    pub fn new(framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>) -> Self {
        let (width, height) = (framebuf.read().unwrap().get_dims().get_width(), framebuf.read().unwrap().get_dims().get_height());
        Self {color_vec:Arc::new(RwLock::new(vec![0 ; width * height])), empty_zbuf:Arc::new(vec![INFINITY ; width * height]),framebuf, zbuf:Arc::new(RwLock::new(SyncUnsafeCell::new(vec![0.0; width * height]))), meshes:Meshes::new(1000, 5), textures:Arc::new(RwLock::new(Textures::new())), latest_camera:Arc::new(RwLock::new(Camera::empty())) }
    }
    pub fn get_read<'a>(&'a self) -> VectorinatorRead<'a> {
        VectorinatorRead {color_vec:self.color_vec.read().unwrap(), empty_zbuf:&self.empty_zbuf, framebuf: self.framebuf.read().unwrap(), zbuf: self.zbuf.read().unwrap(), meshes: self.meshes.get_read(), textures: self.textures.read().unwrap(), camera:self.latest_camera.read().unwrap() }
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

impl IndividualTask for Vectorinator {
    type TD = usize;
    type TID = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => { // Any number of threads
                
                let reader = self.get_read();
                let viewport = self.get_viewport_data();
                reader.render_everything(viewport);
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
            _ => panic!("NO TASK ID BIGGER THAN 4")
        }
    }
}
