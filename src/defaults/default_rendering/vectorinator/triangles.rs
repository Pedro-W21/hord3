use std::{collections::HashMap, simd::{num::{SimdFloat, SimdUint}, Mask, Simd}};

use crate::horde::geometry::{line::Line3D, plane::EquationPlane, rotation::Rotation, vec3d::Vec3Df, HordeFloat, Intersection};

use super::{consts::TABLE_U8_F32, meshes::{InstanceRenderData, MeshTriangles, TrianglePoint}, rendering_spaces::{Vec3DfCam, Vec3DfRaster, ViewportData}, simd_geo::{SIMDRotation, SIMDVec3Df, LANE_COUNT}};

pub struct TransformedMesh {
    pub x:Vec<HordeFloat>,
    pub y:Vec<HordeFloat>,
    pub z:Vec<HordeFloat>,
}

pub type CameraTransformedMesh = TransformedMesh;

impl TransformedMesh {
    pub fn from_realspace<'a>(data:&InstanceRenderData<'a>, camera_pos:&Vec3Df, camera_rotat:&Rotation) -> Self {
        let camera_simd = SIMDVec3Df::from_vec3D(camera_pos);
        let meshpos_simd = SIMDVec3Df::from_vec3D(&data.unique_data.get_pos());
        let final_pos = data.unique_data.get_pos() - camera_pos;
        let final_pos_simd = SIMDVec3Df::from_vec3D(&final_pos);
        let cam_rotat_simd = SIMDRotation::from_rotation(camera_rotat);
        let inst_rotat_simd = SIMDRotation::from_rotation(&data.rotation);
        let mut x = Vec::with_capacity(data.mesh.x.len());
        let mut y = Vec::with_capacity(x.capacity());
        let mut z = Vec::with_capacity(x.capacity());
        let mut simd_part = x.capacity() / LANE_COUNT;
        let mut indices_array = [0;LANE_COUNT];
        for i in 0..LANE_COUNT {
            indices_array[i] = i;
        }
        let mut default = Simd::splat(0.0);
        let mut indices = Simd::from_array(indices_array);
        let mut add_indices = Simd::from_array([LANE_COUNT ; LANE_COUNT]);
        for i in 0..simd_part {
            let vector = SIMDVec3Df::new(
               Simd::gather_or(&data.mesh.x, indices, default),
               Simd::gather_or(&data.mesh.y, indices, default),
               Simd::gather_or(&data.mesh.z, indices, default)
            );
            let (fx, fy, fz) = (cam_rotat_simd.rotate(inst_rotat_simd.rotate(vector) + final_pos_simd.clone())).into_parts();
            x.extend_from_slice(fx.as_array());
            y.extend_from_slice(fy.as_array());
            z.extend_from_slice(fz.as_array());
            indices += add_indices;
        }
        for i in simd_part * LANE_COUNT..x.capacity() {
            let vector = Vec3Df::new(data.mesh.x[i], data.mesh.y[i], data.mesh.z[i]);
            let final_vector = camera_rotat.rotate(data.rotation.rotate(vector + final_pos));
            x.push(final_vector.x);
            y.push(final_vector.y);
            z.push(final_vector.z);
        }
        Self { x, y, z }
    }

    pub fn from_realspace_viewmodel<'a>(data:&InstanceRenderData<'a>) -> Self {
        let meshpos_simd = SIMDVec3Df::from_vec3D(&data.unique_data.get_pos());
        let final_pos = data.unique_data.get_pos();
        let final_pos_simd = SIMDVec3Df::from_vec3D(&final_pos);
        let inst_rotat_simd = SIMDRotation::from_rotation(&data.rotation);
        let mut x = Vec::with_capacity(data.mesh.x.len());
        let mut y = Vec::with_capacity(x.capacity());
        let mut z = Vec::with_capacity(x.capacity());
        let mut simd_part = x.capacity() / LANE_COUNT;
        let mut indices_array = [0;LANE_COUNT];
        for i in 0..LANE_COUNT {
            indices_array[i] = i;
        }
        let mut default = Simd::splat(0.0);
        let mut indices = Simd::from_array(indices_array);
        let mut add_indices = Simd::from_array([LANE_COUNT ; LANE_COUNT]);
        for i in 0..simd_part {
            let vector = SIMDVec3Df::new(
               Simd::gather_or(&data.mesh.x, indices, default),
               Simd::gather_or(&data.mesh.y, indices, default),
               Simd::gather_or(&data.mesh.z, indices, default)
            );
            let (fx, fy, fz) = (inst_rotat_simd.rotate(vector) + final_pos_simd.clone()).into_parts();
            x.extend_from_slice(fx.as_array());
            y.extend_from_slice(fy.as_array());
            z.extend_from_slice(fz.as_array());
            indices += add_indices;
        }
        for i in simd_part * LANE_COUNT..x.capacity() {
            let vector = Vec3Df::new(data.mesh.x[i], data.mesh.y[i], data.mesh.z[i]);
            let final_vector = data.rotation.rotate(vector + final_pos);
            x.push(final_vector.x);
            y.push(final_vector.y);
            z.push(final_vector.z);
        }
        Self { x, y, z }
    }
    pub fn from_realspace_worldpos<'a>(data:&InstanceRenderData<'a>, camera_pos:&Vec3Df, camera_rotat:&Rotation) -> Self {
        let camera_simd = SIMDVec3Df::from_vec3D(camera_pos);
        let cam_rotat_simd = SIMDRotation::from_rotation(camera_rotat);
        let inst_rotat_simd = SIMDRotation::from_rotation(&data.rotation);
        let mut x = Vec::with_capacity(data.mesh.x.len());
        let mut y = Vec::with_capacity(x.capacity());
        let mut z = Vec::with_capacity(x.capacity());
        let mut simd_part = x.capacity() / LANE_COUNT;
        let mut indices_array = [0;LANE_COUNT];
        for i in 0..LANE_COUNT {
            indices_array[i] = i;
        }
        let mut default = Simd::splat(0.0);
        let mut indices = Simd::from_array(indices_array);
        let mut add_indices = Simd::from_array([LANE_COUNT ; LANE_COUNT]);
        for i in 0..simd_part {
            let vector = SIMDVec3Df::new(
               Simd::gather_or(&data.mesh.x, indices, default),
               Simd::gather_or(&data.mesh.y, indices, default),
               Simd::gather_or(&data.mesh.z, indices, default)
            );
            let (fx, fy, fz) = (cam_rotat_simd.rotate(vector - &camera_simd)).into_parts();
            x.extend_from_slice(fx.as_array());
            y.extend_from_slice(fy.as_array());
            z.extend_from_slice(fz.as_array());
            indices += add_indices;
        }
        for i in simd_part * LANE_COUNT..x.capacity() {
            let vector = Vec3Df::new(data.mesh.x[i], data.mesh.y[i], data.mesh.z[i]);
            let final_vector = camera_rotat.rotate(vector - camera_pos);
            x.push(final_vector.x);
            y.push(final_vector.y);
            z.push(final_vector.z);
        }
        Self { x, y, z }
    }
    pub fn get_final_camera_triangles(&self, tris:&MeshTriangles, viewport_data:&ViewportData) -> (TransformedMesh, MeshTriangles) {
        // Not the best implementation right now :(

        let mut new_x = Vec::with_capacity(self.x.capacity() * 2);
        let mut new_y = Vec::with_capacity(self.x.capacity() * 2);
        let mut new_z = Vec::with_capacity(self.x.capacity() * 2);

        let mut new_mesh = TransformedMesh {x:new_x, y:new_y, z:new_z};

        let mut reference_table = HashMap::with_capacity(self.x.capacity());

        let mut new_tris = MeshTriangles::with_capacity(self.x.capacity() * 2);

        for i in 0..tris.len() {
            let tri = tris.get_triangle(&self.x, &self.y, &self.z, i);
            match tri.clip(&viewport_data.camera_plane) {
                (TriangleClip::Inside, _) => {
                    let mut new_indices = [0 ; 3];
                    for (j, index) in tris.get_indices_for_triangle(i).iter().enumerate() {
                        new_indices[j] = get_new_index(tri.get_nth_data(j), *index, &mut reference_table, &mut new_mesh);
                    } 
                    push_new_tri_with_new_indices(tri, new_indices, &mut new_tris);
                },
                (TriangleClip::Outside, _) => (),
                (TriangleClip::OneVertexOut(newer_tris), vertex_out) => {
                    for new_tri in newer_tris {
                        let mut new_indices = [0 ; 3];
                        for (j, index) in tris.get_indices_for_triangle(i).iter().enumerate() {
                            // if j == vertex_out {
                            //    new_indices[j] = get_new_index(tri.get_nth_data(j), *index, &mut reference_table, &mut new_mesh);
                            // }
                            // else {
                                new_indices[j] = add_point_to_mesh(&mut new_mesh, new_tri.get_nth_data(j));
                            // }
                        }
                        push_new_tri_with_new_indices(new_tri, new_indices, &mut new_tris);
                    }
                    
                },
                (TriangleClip::TwoVerticesOut(new_tri), vertex_in) => {
                    let mut new_indices = [0 ; 3];
                    for (j, index) in tris.get_indices_for_triangle(i).iter().enumerate() {
                        if j == vertex_in {
                            new_indices[j] = get_new_index(new_tri.get_nth_data(j), *index, &mut reference_table, &mut new_mesh);
                        }
                        else {
                            new_indices[j] = add_point_to_mesh(&mut new_mesh, new_tri.get_nth_data(j));
                        }
                    }
                    push_new_tri_with_new_indices(new_tri, new_indices, &mut new_tris);
                }
            }
        }

        (
            new_mesh,
            new_tris
        )
    }

    pub fn cameraspace_to_rasterspace(&mut self, viewport_data:&ViewportData, tris:&mut MeshTriangles) {
        let mut simd_part = self.x.capacity() / LANE_COUNT;
        let mut indices_array = [0;LANE_COUNT];
        for i in 0..LANE_COUNT {
            indices_array[i] = i;
        }
        let default = Simd::splat(0.0);
        let mut indices = Simd::from_array(indices_array);
        let add_indices = Simd::from_array([LANE_COUNT ; LANE_COUNT]);
        
        let near_clipping_plane = Simd::splat(viewport_data.near_clipping_plane);
        let half_image_height = Simd::splat(viewport_data.half_image_height);
        let half_image_width = Simd::splat(viewport_data.half_image_width);
        let aspect_ratio = Simd::splat(viewport_data.aspect_ratio);
        let one = Simd::splat(1.0);

        for i in 0..simd_part {
            let mut vector = SIMDVec3Df::new(
                Simd::gather_or(&self.x, indices, default),
                Simd::gather_or(&self.y, indices, default),
                Simd::gather_or(&self.z, indices, default)
            );
            vector.z = one / vector.z;
            vector.x = (one + (near_clipping_plane * vector.x * vector.z)) * half_image_width;
            vector.y = (one - (near_clipping_plane * vector.y * vector.z)) * half_image_height * aspect_ratio;
            vector.x.scatter(&mut self.x, indices);
            vector.y.scatter(&mut self.y, indices);
            vector.z.scatter(&mut self.z, indices);
            
            indices += add_indices;
        }
        for i in simd_part * LANE_COUNT..self.x.len() {
            let vector = Vec3Df::new(self.x[i], self.y[i], self.z[i]);
            let final_vector = Vec3DfRaster::from_cameraspace(Vec3DfCam(vector), viewport_data);
            self.x[i] = final_vector.x;
            self.y[i] = final_vector.y;
            self.z[i] = final_vector.z;
        }

        for tri in 0..tris.len() {
            let (p1, p2, p3) = tris.get_triangle_uvs(tri);
            *p1.1 = *p1.1 * self.z[p1.0];
            *p1.2 = *p1.2 * self.z[p1.0];
            *p2.1 = *p2.1 * self.z[p2.0];
            *p2.2 = *p2.2 * self.z[p2.0];
            *p3.1 = *p3.1 * self.z[p3.0];
            *p3.2 = *p3.2 * self.z[p3.0];
        }
    }
}

fn get_new_index(point:&TrianglePointData, old_index:usize, reference:&mut HashMap<usize, usize>, tf_mesh:&mut TransformedMesh) -> usize {
    match reference.get(&old_index) {
        Some(new_ref) => *new_ref,
        None => {
            let new_index = tf_mesh.x.len();
            tf_mesh.x.push(point.pos.x);
            tf_mesh.y.push(point.pos.y);
            tf_mesh.z.push(point.pos.z);
            reference.insert(old_index, new_index);
            new_index
        }
    }
}

fn get_iter_tris(non_iter:usize) -> [usize ; 2] {
    match non_iter {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => panic!("IMPOSSIBLE INDEX")
    }
}

fn add_point_to_mesh(mesh:&mut TransformedMesh, point:&TrianglePointData) -> usize {
    let index = mesh.x.len();
    mesh.x.push(point.pos.x);
    mesh.y.push(point.pos.y);
    mesh.z.push(point.pos.z);
    index
}

fn push_new_tri_with_new_indices(tri:SingleFullTriangle, indices:[usize ; 3], tris:&mut MeshTriangles) {
    tris.add_triangle(
        TrianglePoint::new(
            indices[0],
            tri.p1.u, tri.p1.v, tri.p1.r, tri.p1.g, tri.p1.b
        ),
        TrianglePoint::new(
            indices[1],
            tri.p2.u, tri.p2.v, tri.p2.r, tri.p2.g, tri.p2.b
        ),
        TrianglePoint::new(
            indices[2],
            tri.p3.u, tri.p3.v, tri.p3.r, tri.p3.g, tri.p3.b
        ),
        tri.texture_flags.0,
        tri.texture_flags.1
    )
}

pub enum TriangleClip {
    Inside,
    Outside,
    OneVertexOut([SingleFullTriangle ; 2]),
    TwoVerticesOut(SingleFullTriangle),
}

pub struct SingleFullTriangle {
    pub p1:TrianglePointData,
    pub p2:TrianglePointData,
    pub p3:TrianglePointData,
    pub texture_flags:(u32, u32)
}
impl SingleFullTriangle {
    pub fn new(p1d:TrianglePointData, p2d:TrianglePointData, p3d:TrianglePointData, texture_flags:(u32, u32)) -> Self {
        Self {p1: p1d, p2: p2d, p3: p3d, texture_flags }
    }
    pub fn get_nth_data(&self, n:usize) -> &TrianglePointData {
        match n {
            0 => &self.p1,
            1 => &self.p2,
            2 => &self.p3,
            _ => panic!("IMPOSSIBLE")
        }
    }
    pub fn get_area(&self) -> HordeFloat {
        (self.p3.pos.0 - self.p1.pos.0).det2D(&(self.p2.pos.0 - self.p1.pos.0))
    }
    pub fn clip(&self, plane:&EquationPlane) -> (TriangleClip, usize) {
        let d1 = plane.signed_distance(&self.p1.pos);
        let d2 = plane.signed_distance(&self.p2.pos);
        let d3 = plane.signed_distance(&self.p3.pos);
        if d1 > 0.0 && d2 > 0.0 && d3 > 0.0 {
            //dbg!(d1, d2, d3);
            (TriangleClip::Inside, 0)
        }
        else if d1 < 0.0 && d2 < 0.0 && d3 < 0.0 {
            (TriangleClip::Outside, 0)
        }
        else if d1 > 0.0 && d2 < 0.0 && d3 < 0.0 {
            (self.clip_1_in(&self.p1, &self.p2, &self.p3, &plane), 0)
        }
        else if d1 < 0.0 && d2 > 0.0 && d3 < 0.0 {
            (self.clip_1_in(&self.p2, &self.p3, &self.p1,  &plane), 1)
        }
        else if d1 < 0.0 && d2 < 0.0 && d3 > 0.0 {
            (self.clip_1_in(&self.p3, &self.p1, &self.p2,  &plane), 2)
        }
        else if d1 > 0.0 && d2 > 0.0 {
            (self.clip_2_in(&self.p1, &self.p2, &self.p3,  &plane), 2)
        }
        else if d1 > 0.0 && d3 > 0.0 {
            (self.clip_2_in(&self.p3, &self.p1, &self.p2,  &plane), 1)
        }
        else if d2 > 0.0 && d3 > 0.0 {
            (self.clip_2_in(&self.p2, &self.p3, &self.p1,  &plane), 0)
        }
        else {
            (TriangleClip::Outside, 0)
        }
    }
    fn clip_1_in(&self, a:&TrianglePointData, b:&TrianglePointData, c:&TrianglePointData, plane:&EquationPlane) -> TriangleClip {
        let ab = Line3D::new(*a.pos.clone(), *b.pos.clone() - *a.pos.clone());
        let ac = Line3D::new(*a.pos.clone(), *c.pos.clone() - *a.pos.clone());
        let b_p = plane.intersect_with(&ab);
        let c_p = plane.intersect_with(&ac);
        let b_p_coef = b_p.unwrap_coef();
        let one_m_b_p_coef = 1.0 - b_p_coef;
        let c_p_coef = c_p.unwrap_coef();
        let one_m_c_p_coef = 1.0 - c_p_coef;
        let collux_2 = interpolate_collux((a.r, a.g, a.b), (b.r, b.g, b.b), b_p_coef, one_m_b_p_coef);
        let u_v_2 = interpolate_uv((a.u, a.v), (b.u, b.v), b_p_coef, one_m_b_p_coef);
        let collux_3 = interpolate_collux((a.r, a.g, a.b), (c.r, c.g, c.b), c_p_coef, one_m_c_p_coef);
        let u_v_3 = interpolate_uv((a.u, a.v), (c.u, c.v), c_p_coef, one_m_c_p_coef);
        TriangleClip::TwoVerticesOut(SingleFullTriangle::new(
            TrianglePointData::new(a.pos.clone(), a.u, a.v, a.r, a.g, a.b),
            TrianglePointData::new(Vec3DfCam(b_p.to_point(&ab)), u_v_2.0, u_v_2.1, collux_2.0, collux_2.1, collux_2.2),
            TrianglePointData::new(Vec3DfCam(c_p.to_point(&ac)), u_v_3.0, u_v_3.1, collux_3.0, collux_3.1, collux_3.2),
        self.texture_flags
        ))
    }
    pub fn bounding_box(&self, image_width:f32, image_height:f32) -> ((i32, i32), (i32, i32)) {
        (
            (
                self.p1.pos.x.min(self.p2.pos.x.min(self.p3.pos.x)).max(0.0).min(image_width) as i32,
                self.p1.pos.y.min(self.p2.pos.y.min(self.p3.pos.y)).max(0.0).min(image_height) as i32
            ),
            (
                self.p1.pos.x.max(self.p2.pos.x.max(self.p3.pos.x)).min(image_width).max(0.0) as i32,
                (self.p1.pos.y.max(self.p2.pos.y.max(self.p3.pos.y)) + 1.0).min(image_height).max(0.0) as i32
                // + 1 rajouté pour éviter lignes noires
            )
        )
    }
    fn clip_2_in(&self, a:&TrianglePointData, b:&TrianglePointData, c:&TrianglePointData, plane:&EquationPlane) -> TriangleClip {
        let ac = Line3D::new(*a.pos.clone(), *c.pos.clone() - *a.pos.clone());
        let bc = Line3D::new(*b.pos.clone(), *c.pos.clone() - *b.pos.clone());
        let a_p = plane.intersect_with(&ac);
        let b_p = plane.intersect_with(&bc);
        let a_p_point = a_p.to_point(&ac);
        
        let b_p_coef = b_p.unwrap_coef();
        let one_m_b_p_coef = 1.0 - b_p_coef;
        
        let a_p_coef = a_p.unwrap_coef();
        let one_m_a_p_coef = 1.0 - a_p_coef;
        let a_p_collux = interpolate_collux((a.r, a.g, a.b), (c.r, c.g, c.b), a_p_coef, one_m_a_p_coef);
        let a_p_uv = interpolate_uv((a.u, a.v), (c.u, c.v), a_p_coef, one_m_a_p_coef);

        let cb_collux = interpolate_collux((b.r, b.g, b.b), (c.r, c.g, c.b), b_p_coef, one_m_b_p_coef);
        let cb_uv = interpolate_uv((b.u, b.v), (c.u, c.v), b_p_coef, one_m_b_p_coef);
        TriangleClip::OneVertexOut(
        [
            SingleFullTriangle::new(
                a.clone(),
                b.clone(),
                TrianglePointData::new(Vec3DfCam(a_p_point.clone()), a_p_uv.0, a_p_uv.1, a_p_collux.0, a_p_collux.1, a_p_collux.2),
            self.texture_flags
        ),
        SingleFullTriangle::new(
            TrianglePointData::new(Vec3DfCam(a_p_point.clone()), a_p_uv.0, a_p_uv.1, a_p_collux.0, a_p_collux.1, a_p_collux.2),
            b.clone(),
            TrianglePointData::new(Vec3DfCam(b_p.to_point(&bc)), cb_uv.0, cb_uv.1, cb_collux.0, cb_collux.1, cb_collux.2),
            self.texture_flags
        )
        ])
    }
}


#[derive(Clone)]
pub struct TrianglePointData {
    pub pos:Vec3DfCam,
    pub u:HordeFloat,
    pub v:HordeFloat,
    pub r:u8,
    pub g:u8,
    pub b:u8
}

impl TrianglePointData {
    pub fn new(pos:Vec3DfCam, u:HordeFloat, v:HordeFloat, r:u8, g:u8, b:u8) -> Self {
        Self {pos, u, v, r, g, b }
    }
}

fn interpolate_collux(start:(u8,u8,u8), stop:(u8,u8,u8), coef:f32, one_m_coef:f32) -> (u8,u8,u8) {
    let start_f = collux_u8_a_f32(start);
    let stop_f = collux_u8_a_f32(stop);
    collux_f32_a_u8((start_f.0 * one_m_coef + stop_f.0 * coef, start_f.1 * one_m_coef + stop_f.1 * coef, start_f.2 * one_m_coef + stop_f.2 * coef))

}

pub fn collux_u8_a_f32(collux: (u8, u8, u8)) -> (f32, f32, f32) {
    (
        TABLE_U8_F32[collux.0 as usize],
        TABLE_U8_F32[collux.1 as usize],
        TABLE_U8_F32[collux.2 as usize],
    )
}

pub fn collux_u8_tuple_to_f32_simd(collux: (u8, u8, u8), level:u8) -> Simd<f32, 4> {
    Simd::from_array(
        [
            TABLE_U8_F32[collux.0 as usize],
            TABLE_U8_F32[collux.1 as usize],
            TABLE_U8_F32[collux.2 as usize],
            TABLE_U8_F32[level as usize]
        ]
    )
}

pub fn collux_one_simd_to_u8_level(collux:Simd<f32, 4>) -> ((u8,u8,u8), u8) {
    let array = (collux * Simd::splat(255.0)).to_array();
    (
        (
            array[0] as u8,
            array[1] as u8,
            array[2] as u8
        ),
        array[3] as u8,
    )
}

pub fn collux_u8_to_f32_simd(collux: (Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>)) -> (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>,Simd<f32, LANE_COUNT>) {
    (
        Simd::gather_or(&TABLE_U8_F32, collux.0.cast(), Simd::splat(1.0)),
        Simd::gather_or(&TABLE_U8_F32, collux.1.cast(), Simd::splat(1.0)),
        Simd::gather_or(&TABLE_U8_F32, collux.2.cast(), Simd::splat(1.0)),
    )
}

pub fn color_u32_to_u8_simd(color:Simd<u32, LANE_COUNT>) -> (Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>) {
    (
        ((color >> 16) & Simd::splat(255)).cast::<u8>(),
        ((color >> 8) & Simd::splat(255)).cast::<u8>(),
        (color).cast::<u8>() 
    )
}

pub fn color_u32_seperate(color:Simd<u32, LANE_COUNT>) -> (Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>) {
    (
        ((color >> 16) & Simd::splat(255)),
        ((color >> 8) & Simd::splat(255)),
        (color) 
    )
}

pub fn mul_u32_color_and_divide(color:Simd<u32, LANE_COUNT>, other_color:Simd<u32, LANE_COUNT>) -> Simd<u32, LANE_COUNT> {
    ((color * other_color) >> 8) & Simd::splat(255)
}

pub fn simd_u32_rgb_to_argb(col: (Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>)) -> Simd<u32, LANE_COUNT> {
    (col.0 << 16) + (col.1 << 8) + (col.2)
}

pub fn collux_f32_a_u8(collux: (f32, f32, f32)) -> (u8, u8, u8) {
    (
        (collux.0 * 255.0) as u8,
        (collux.1 * 255.0) as u8,
        (collux.2 * 255.0) as u8,
    )
}

pub fn collux_f32_to_u8_simd(collux: (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>)) -> (Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>) {
    (
        (collux.0 * Simd::splat(255.0)).cast(),
        (collux.1 * Simd::splat(255.0)).cast(),
        (collux.2 * Simd::splat(255.0)).cast(),
    )
}

pub fn simd_f32_to_u32_color(col: (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>)) -> Simd<u32, LANE_COUNT> {
    ((col.0 * Simd::splat(255.0)).cast::<u32>() << 16) + ((col.1 * Simd::splat(255.0)).cast::<u32>() << 8) + ((col.2 * Simd::splat(255.0)).cast::<u32>())
}

pub fn simd_rgb_to_argb(col: (Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>, Simd<u8, LANE_COUNT>)) -> Simd<u32, LANE_COUNT> {
    (col.0.cast::<u32>() << 16) + (col.1.cast::<u32>() << 8) + (col.2.cast::<u32>())
}

fn interpolate_uv(start:(f32,f32), stop:(f32,f32), coef:f32, one_m_coef:f32) -> (f32, f32) {
    (start.0 * one_m_coef + stop.0 * coef, start.1 * one_m_coef + stop.1 * coef)
}