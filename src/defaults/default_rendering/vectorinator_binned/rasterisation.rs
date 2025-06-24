use std::{f32::INFINITY, simd::{cmp::{SimdOrd, SimdPartialOrd}, num::{SimdFloat, SimdInt}, Mask, Simd, StdFloat}};

use crate::horde::{frontend::{HordeWindowDimensions, SyncUnsafeHordeFramebuffer}, geometry::{vec3d::Vec3Df, HordeFloat}};

use super::{bins::{Bin, PreCalcdData}, meshes::Rectangle, rendering_spaces::ViewportData, shaders::ShaderData, simd_geo::{SIMDVec2Df, SIMDVec3Df, LANE_COUNT, LANE_COUNT_F32, LANE_COUNT_I32, LANE_COUNT_U32}, textures::{argb_to_rgb, rgb_to_argb, MipMap, Textures}, triangles::{collux_f32_a_u8, collux_f32_to_u8_simd, collux_u8_a_f32, collux_u8_to_f32_simd, color_u32_seperate, color_u32_to_u8_simd, mul_u32_color_and_divide, simd_f32_to_u32_color, simd_rgb_to_argb, simd_u32_rgb_to_argb, SingleFullTriangle}, VectorinatorRead};

pub type SimdPixel = SIMDVec3Df;

pub struct InternalRasterisationData<'a> {
    frambuf:&'a mut Vec<u32>,
    zbuf:&'a mut Vec<f32>,
    nbuf:&'a mut Vec<u32>,
    dims:HordeWindowDimensions,
    pub textures:&'a Textures,
    viewport_data:ViewportData
}

impl<'a> InternalRasterisationData<'a> {
    pub fn from_vectorinator_read<SD:ShaderData>(read:&'a VectorinatorRead<'a,SD>, viewport_data:ViewportData) -> Self {
        unsafe {
            Self { frambuf: &mut *read.internal_framebuf.get_data_cell().get(), zbuf: &mut *read.zbuf.get(), nbuf: &mut *read.nbuf.get(), dims: read.framebuf.get_dims(), textures: &read.textures, viewport_data}
        }
        
    }
    pub fn copy_from_bin(&mut self, bin:&Bin) {
        unsafe {
            let bin_image_data = bin.image_data.get().as_ref().unwrap_unchecked();
            let width = bin_image_data.bin_size_i;
            let y_end = bin.end_y_i.min(self.dims.get_height_i() as i32);
            let x_end = bin.end_x_i.min(self.dims.get_width_i() as i32);
            let real_width = (x_end - bin.start_x_i) as usize;
            for i in bin.start_y_i..y_end {
                let y = (i * self.dims.get_width_i() as i32) as usize;
                let y_bin = ((i - bin.start_y_i) * width) as usize;
                let (start, end) = (y + bin.start_x_i as usize, y + x_end as usize);
                let range = start..end;
                let range_bin = y_bin..(y_bin + real_width);
                assert!(range.len() == range_bin.len());
                self.frambuf[range.clone()].copy_from_slice(&bin_image_data.frambuf[range_bin.clone()]);
                self.zbuf[range.clone()].copy_from_slice(&bin_image_data.zbuf[range_bin.clone()]);
                self.nbuf[range.clone()].copy_from_slice(&bin_image_data.nbuf[range_bin.clone()]);
            }
        }
    }
    pub fn clear_framebuf(&mut self, color_vec:&Vec<u32>) {
        self.frambuf.copy_from_slice(&color_vec);
    }
    pub fn clear_zbuf(&mut self, copy_vec:&Vec<f32>) {
        self.zbuf.copy_from_slice(&copy_vec);
    }
    pub fn clear_nbuf(&mut self, copy_vec:&Vec<u32>) {
        self.nbuf.copy_from_slice(&copy_vec);
    }
}


pub fn rasterise_if_possible<'a>(triangle:&(SingleFullTriangle, PreCalcdData), data:&InternalRasterisationData<'a>, bin:&Bin) {
    rasterise_any_collux(&triangle.0, data, &triangle.1, bin);
}

fn rasterise_any_collux<'a>(triangle:&SingleFullTriangle, data:&InternalRasterisationData<'a>, pre_data:&PreCalcdData, bin:&Bin) {
    let bounding_box = triangle.complete_bounding_box( bin, pre_data.bounding_box_pre_binned);
    
    let x_dist_to_border = (data.dims.get_width() as i32).abs_diff(bounding_box.0.0.max(bounding_box.1.0)) as i32;
    
    if x_dist_to_border > LANE_COUNT_I32 && pre_data.area > 50.0 && false {
        //full_normal_tri(triangle, data, area, bounding_box);
        full_simd_tri(triangle, data, pre_data, bounding_box, x_dist_to_border as usize, bin);
    }
    else {
        //dbg!(bounding_box);
        full_normal_tri(triangle, data, pre_data, bounding_box, bin);
    }
}

fn full_normal_tri<'a>(triangle:&SingleFullTriangle, data:&InternalRasterisationData<'a>, pre_data:&PreCalcdData, bounding_box:((i32, i32), (i32, i32)), bin:&Bin) {
    let texture = data.textures.get_text_with_id(triangle.texture_flags.0 as usize);
    let len = texture.get_mip_map(0).data.len();
    let mip_map = ((len as f32 * pre_data.inv_area).log2()) as usize;
    let t_len = texture.get_mip_map(mip_map).len_m1;

    let x_diff = ((bounding_box.1.0 + 1).min(data.dims.get_width_i() as i32 - 1).min(bin.end_x_i) - bounding_box.0.0);
    let y_diff = bounding_box.1.1 - bounding_box.0.1;
    let mip_map = texture.get_mip_map(mip_map);
    let tri = TriangleData::from_tri_area(&triangle, pre_data, mip_map);
    let start_x = bounding_box.0.0 as f32 + 0.5;
    let mut point = Vec3Df::new(start_x, bounding_box.0.1 as f32 + 0.5, 0.0);
    let mut collux = (triangle.p1.r, triangle.p1.g, triangle.p1.b);
    let float_collux = collux_u8_a_f32(collux);
    unsafe {
        let image_data = bin.image_data.get().as_mut().unwrap_unchecked();
        let start_x_usize = (bounding_box.0.0 - bin.start_x_i) as usize;
        let mut start_y = (bounding_box.0.1 - bin.start_y_i) as usize * image_data.bin_size;
        //dbg!(bounding_box, bin.start_x_i, bin.start_y_i, start_x_usize, start_y, x_diff, y_diff, image_data.bin_size);
        for y in 0..y_diff {
            let mut pixel = (start_x_usize + start_y);
            point.x = start_x;
            for x in 0..x_diff {
                let (w0, w1, w2, z, is_in) = tri.calc_w0_w1_w2_z_is_in(&point);
                if is_in && z < image_data.zbuf[pixel] {
                    *image_data.zbuf.get_unchecked_mut(pixel) = z;
                    *image_data.nbuf.get_unchecked_mut(pixel) = pre_data.packed_normal;
                    let (u, v) = tri.calc_xi_yi(w0, w1, w2, z);
                    let texture_pixel = collux_u8_a_f32(argb_to_rgb(mip_map.data[(v.to_int_unchecked::<usize>() * mip_map.largeur_usize + u.to_int_unchecked::<usize>()).clamp(0, t_len)]));
                    let final_color = rgb_to_argb(collux_f32_a_u8((float_collux.0 * texture_pixel.0, float_collux.1 * texture_pixel.1, float_collux.2 * texture_pixel.2)));
                    *image_data.frambuf.get_unchecked_mut(pixel) = final_color;
                    // dbg!(pixel);
                }
                point.x += 1.0;
                pixel += 1;
            }
            point.y += 1.0;
            start_y += image_data.bin_size;
        }
    }
    
}

fn full_simd_tri<'a>(triangle:&SingleFullTriangle, data:&InternalRasterisationData<'a>, pre_data:&PreCalcdData, mut bounding_box:((i32, i32), (i32, i32)), dist_to_border:usize, bin:&Bin) {
    //println!("DOING A TRIANGLE");
    
    
    bounding_box.0.0 = bounding_box.0.0 - bounding_box.0.0 % LANE_COUNT_I32;
    let y_diff = bounding_box.1.1 - bounding_box.0.1;
    let texture = data.textures.get_text_with_id(triangle.texture_flags.0 as usize);
    let xs_f = bounding_box.0.0 as f32;
    let mut x_array = [0.0 ; LANE_COUNT];
    let ys_f = bounding_box.0.1 as f32;
    for i in 0..LANE_COUNT {
        x_array[i] = xs_f + i as f32 + 0.5;
    }
    let x_diff = bounding_box.1.0 - bounding_box.0.0;

    let x_simd_part = x_diff / LANE_COUNT_I32 + 1;
    let mip_map = texture.get_mip_map(pre_data.mip_map);
    
    let addition_vector = Simd::splat(LANE_COUNT_F32);
    let x_simd = Simd::from_array(x_array);
    //let image_width_vector = Simd::splat(data.viewport_data.image_width as usize);
    let tri = SIMDTriangleData::from_tri_area(&triangle, pre_data, mip_map);
    let mut point = SIMDVec2Df::new(x_simd, Simd::splat(ys_f + 0.5));
    let t_len_simd = Simd::splat(mip_map.len_m1);
    //if x_diff > data.dims.get_width() as i32 || y_diff > data.dims.get_height() as i32 {
    //    dbg!(x_diff, y_diff);
    //}
    unsafe {
        let image_data = bin.image_data.get().as_mut().unwrap_unchecked();
        let start_x_usize = (point.x - Simd::splat(bin.start_x_f)).to_int_unchecked::<usize>();
        let mut start_y = Simd::splat((ys_f - bin.start_y_f) as usize * image_data.bin_size);
        
        'fory : for y in 0..y_diff {
            point.x = x_simd;
            let mut pixel = (start_y  + start_x_usize);
            let mut was_inside = false;
            for x in 0..x_simd_part {
                let (w0, w1, w2, z, in_mask) = tri.calc_w0_w1_w2_z_is_in(&point);
                
                if in_mask.any() {
                    let zbuf_mask = Simd::gather_select_unchecked(&data.zbuf, Mask::splat(true), pixel, Simd::splat(INFINITY)).simd_ge(z);
                    was_inside = true;
                    if zbuf_mask.any() {
                        let final_mask:Mask<isize, LANE_COUNT> = ((zbuf_mask & in_mask)).into();
                        z.scatter_select_unchecked(&mut image_data.zbuf, final_mask, pixel);
                        Simd::splat(pre_data.packed_normal).scatter_select_unchecked(&mut image_data.nbuf, final_mask, pixel);
                        let (u, v) = tri.calc_xi_yi(w0, w1, w2, z);
                        let texture_pixel = color_u32_seperate(Simd::gather_select_unchecked(&mip_map.data, Mask::splat(true), (v.to_int_unchecked::<usize>() * tri.texture_width_usize + u.to_int_unchecked::<usize>()).simd_min(t_len_simd), Simd::splat(0)));
                        let final_color = simd_u32_rgb_to_argb((mul_u32_color_and_divide(tri.collux.0, texture_pixel.0), mul_u32_color_and_divide(tri.collux.1, texture_pixel.1), mul_u32_color_and_divide(tri.collux.2, texture_pixel.2)));

                        /* u8 and float impl :
                        let texture_pixel = collux_u8_to_f32_simd(color_u32_to_u8_simd(Simd::gather_select_unchecked(&mip_map.data, Mask::splat(true), (v.to_int_unchecked::<usize>() * tri.texture_width_usize + u.to_int_unchecked::<usize>()).simd_min(t_len_simd), Simd::splat(0))));
                        let final_color = simd_f32_to_u32_color((tri.collux.0 * texture_pixel.0, tri.collux.1 * texture_pixel.1, tri.collux.2 * texture_pixel.2));

                        */
                        final_color.scatter_select_unchecked(&mut image_data.frambuf, final_mask, pixel);
                    }
                }
                else if was_inside {
                    point.y += Simd::splat(1.0);
                    start_y += Simd::splat(image_data.bin_size);
                    continue 'fory;
                }
                point.x += addition_vector;
                pixel += Simd::splat(LANE_COUNT);
            }
            point.y += Simd::splat(1.0);
            start_y += Simd::splat(image_data.bin_size);
        }
    }

    
}

struct SIMDTriangleData {
    p1_simd:SIMDVec3Df,
    p2_simd:SIMDVec3Df,
    p3_simd:SIMDVec3Df,
    inv_p1_z:Simd<f32, LANE_COUNT>,
    inv_p2_z:Simd<f32, LANE_COUNT>,
    inv_p3_z:Simd<f32, LANE_COUNT>,
    u1:Simd<f32, LANE_COUNT>,
    u2:Simd<f32, LANE_COUNT>,
    u3:Simd<f32, LANE_COUNT>,
    v1:Simd<f32, LANE_COUNT>,
    v2:Simd<f32, LANE_COUNT>,
    v3:Simd<f32, LANE_COUNT>,
    collux:(Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>, Simd<u32, LANE_COUNT>),
    area:Simd<f32, LANE_COUNT>,
    texture_width:Simd<f32, LANE_COUNT>,
    texture_height:Simd<f32, LANE_COUNT>,
    texture_width_usize:Simd<usize, LANE_COUNT>,
}

impl SIMDTriangleData {
    pub fn from_tri_area(triangle:&SingleFullTriangle, pre_data:&PreCalcdData, texture:&MipMap) -> Self {
        Self {
            p1_simd: SIMDVec3Df::from_vec3D(&triangle.p1.pos),
            p2_simd: SIMDVec3Df::from_vec3D(&triangle.p2.pos),
            p3_simd: SIMDVec3Df::from_vec3D(&triangle.p3.pos),
            inv_p1_z: Simd::splat(1.0/triangle.p1.pos.z),
            inv_p2_z: Simd::splat(1.0/triangle.p2.pos.z),
            inv_p3_z: Simd::splat(1.0/triangle.p3.pos.z),
            u1: Simd::splat(triangle.p1.u),
            u2: Simd::splat(triangle.p2.u),
            u3: Simd::splat(triangle.p3.u),
            v1: Simd::splat(triangle.p1.v),
            v2: Simd::splat(triangle.p2.v),
            v3: Simd::splat(triangle.p3.v),
            collux:(Simd::splat(triangle.p1.r as u32), Simd::splat(triangle.p1.g as u32), Simd::splat(triangle.p1.b as u32)),
            area: Simd::splat(pre_data.inv_area),
            texture_height:Simd::splat(texture.hauteur),
            texture_width:Simd::splat(texture.largeur),
            texture_width_usize:Simd::splat(texture.largeur_usize),
        }
    }
    pub fn calc_w0_w1_w2_z_is_in(&self, point:&SIMDVec2Df) -> (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Mask<i32, LANE_COUNT>) {
        let w0 = edge_function_simd(&self.p2_simd,&self.p3_simd, point) * self.area;
        let w1 = edge_function_simd(&self.p3_simd, &self.p1_simd, point) * self.area;
        let w2 = edge_function_simd(&self.p1_simd, &self.p2_simd, point) * self.area;
        
        let is_in = w0.simd_ge(Simd::splat(0.0)) & w1.simd_ge(Simd::splat(0.0)) & w2.simd_ge(Simd::splat(0.0));
        
        (w0, w1, w2, w0 * self.inv_p1_z + w1 * self.inv_p2_z + w2 * self.inv_p3_z, is_in)
    }
    pub fn calc_xi_yi(&self, w0:Simd<f32, LANE_COUNT>, w1:Simd<f32, LANE_COUNT>, w2:Simd<f32, LANE_COUNT>, z:Simd<f32, LANE_COUNT>) -> (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>) {
        let z = Simd::splat(1.0)/(w0 * self.p1_simd.z + w1 * self.p2_simd.z + w2 * self.p3_simd.z);
        (
            (self.u1 * w0 + self.u2 * w1 + self.u3 * w2) * z * self.texture_width,
            (self.v1 * w0 + self.v2 * w1 + self.v3 * w2) * z * self.texture_height,
        )
    }
    pub fn calc_xi_yi_z_is_in(&self, point:&SIMDVec2Df) -> (Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Simd<f32, LANE_COUNT>, Mask<i32, LANE_COUNT>) {
        let w0 = edge_function_simd(&self.p2_simd,&self.p3_simd, point) * self.area;
        let w1 = edge_function_simd(&self.p3_simd, &self.p1_simd, point) * self.area;
        let w2 = edge_function_simd(&self.p1_simd, &self.p2_simd, point) * self.area;
        let in0 = w0.simd_ge(Simd::splat(0.0));
        let in1 = w1.simd_ge(Simd::splat(0.0));
        let in2 = w2.simd_ge(Simd::splat(0.0));
        
        let is_in = w0.simd_ge(Simd::splat(0.0)) & w1.simd_ge(Simd::splat(0.0)) & w2.simd_ge(Simd::splat(0.0));
        if is_in.any() {
            let z = Simd::splat(1.0)/(w0 * self.p1_simd.z + w1 * self.p2_simd.z + w2 * self.p3_simd.z);
            (
                (self.u1 * w0 + self.u2 * w1 + self.u3 * w2) * z * self.texture_width,
                (self.v1 * w0 + self.v2 * w1 + self.v3 * w2) * z * self.texture_height,
                w0 * self.inv_p1_z + w1 * self.inv_p2_z + w2 * self.inv_p3_z,
                is_in
            )
            
        }
        else {
            (
                Simd::splat(0.0),
                Simd::splat(0.0),
                Simd::splat(0.0),
                is_in
            )
        }
    }
}

pub fn edge_function_simd(v1:&SIMDVec3Df, v2:&SIMDVec3Df, v3:&SIMDVec2Df) -> Simd<f32, LANE_COUNT> {
    (v3.x - v1.x) * (v2.y - v1.y) - (v3.y - v1.y) * (v2.x - v1.x)
}

pub fn edge_function(v1:&Vec3Df, v2:&Vec3Df, v3:&Vec3Df) -> f32 {
    (v3.x - v1.x) * (v2.y - v1.y) - (v3.y - v1.y) * (v2.x - v1.x)
}

pub struct TriangleData {
    p1_simd:Vec3Df,
    p2_simd:Vec3Df,
    p3_simd:Vec3Df,
    inv_p1_z:f32,
    inv_p2_z:f32,
    inv_p3_z:f32,
    u1:f32,
    u2:f32,
    u3:f32,
    v1:f32,
    v2:f32,
    v3:f32,
    area:f32,
    texture_width:f32,
    texture_height:f32,
}

impl TriangleData {
    pub fn from_tri_area(triangle:&SingleFullTriangle, pre_data:&PreCalcdData, texture:&MipMap) -> Self {
        Self {
            p1_simd: triangle.p1.pos.0.clone(),
            p2_simd: triangle.p2.pos.0.clone(),
            p3_simd: triangle.p3.pos.0.clone(),
            inv_p1_z: 1.0/triangle.p1.pos.z,
            inv_p2_z: 1.0/triangle.p2.pos.z,
            inv_p3_z: 1.0/triangle.p3.pos.z,
            u1: triangle.p1.u,
            u2: triangle.p2.u,
            u3: triangle.p3.u,
            v1: triangle.p1.v,
            v2: triangle.p2.v,
            v3: triangle.p3.v,
            area: pre_data.inv_area,
            texture_height:texture.hauteur,
            texture_width:texture.largeur
        }
    }
    pub fn calc_xi_yi_z_is_in(&self, point:&Vec3Df) -> (f32, f32, f32, bool) {
        let w0 = edge_function(&self.p2_simd,&self.p3_simd, point) * self.area;
        let w1 = edge_function(&self.p3_simd, &self.p1_simd, point) * self.area;
        let w2 = edge_function(&self.p1_simd, &self.p2_simd, point) * self.area;
        let is_in = (w0 > 0.0) & (w1 > 0.0) & (w2 > 0.0);
        if is_in {
            let z = 1.0/(w0 * self.p1_simd.z + w1 * self.p2_simd.z + w2 * self.p3_simd.z);
            (
            (self.u1 * w0 + self.u2 * w1 + self.u3 * w2) * z * self.texture_width,
            (self.v1 * w0 + self.v2 * w1 + self.v3 * w2) * z * self.texture_height,
            w0 * self.inv_p1_z + w1 * self.inv_p2_z + w2 * self.inv_p3_z,
            is_in
            )
        }   
        else {
            (0.0, 0.0, 0.0, is_in)
        }
    }

    pub fn calc_xi_yi(&self, w0:f32, w1:f32, w2:f32, z:f32) -> (f32, f32) {
        (
            (self.u1 * w0 + self.u2 * w1 + self.u3 * w2) * z * self.texture_width,
            (self.v1 * w0 + self.v2 * w1 + self.v3 * w2) * z * self.texture_height,
        )
    }

    pub fn calc_w0_w1_w2_z_is_in(&self, point:&Vec3Df) -> (f32,f32,f32,f32, bool) {
        let w0 = edge_function(&self.p2_simd,&self.p3_simd, point) * self.area;
        if w0 >= 0.0 {
            let w1 = edge_function(&self.p3_simd, &self.p1_simd, point) * self.area;
            if w1 >= 0.0 {
                let w2 = edge_function(&self.p1_simd, &self.p2_simd, point) * self.area;
                if w2 >= 0.0 {
                    return (w0, w1, w2, w0 * self.inv_p1_z + w1 * self.inv_p2_z + w2 * self.inv_p3_z, true)
                }
            }
        }
        (0.0, 0.0, 0.0, 0.0, false)
        
    }
}


pub fn textured_rect_at_depth<'a>(mut dims:Rectangle<i32>, depth:f32, texture:u16, data:&mut InternalRasterisationData<'a>, collux:(u8,u8,u8), viewport_data:&ViewportData) {
    if dims.width() > 0 && dims.height() > 0 {
        let collux_float = collux_u8_a_f32(collux);
        let data_text = data.textures.get_text_with_id(texture as usize);
        let text = data_text.get_mip_map(0);
        let text_data = &text.data;
        let mut tx = 0.0;
        let mut ty = 0.0;
        let dtx = (text.largeur/dims.width() as f32).clamp(0.0, text.largeur - f32::EPSILON);
        let dty = (text.hauteur/dims.height() as f32).clamp(0.0, text.hauteur - f32::EPSILON);
        let mut tx_start = 0.0;
        let mut posbuf = 0;
        let v_range = dims.v_range();
        let h_range = dims.h_range();
        
        let v_start = if v_range.start > 0 {
            v_range.start
        }
        else {
            ty = -v_range.start as f32 * dtx;
            0
        };
        let v_end = if v_range.end <= viewport_data.image_height as i32 {
            v_range.end
        }
        else {
            viewport_data.image_height as i32
        };

        let h_start = if h_range.start > 0 {
            h_range.start
        }
        else {
            tx_start = -h_range.start as f32 * dtx;
            0
        };
        let h_end = if h_range.end <= viewport_data.image_width as i32 {
            h_range.end
        }
        else {
            viewport_data.image_width as i32
        };
        //dbg!(normal);
        unsafe{
            let mut framebuf = &mut data.frambuf;
            let mut zbuf = &mut data.zbuf;
            match data_text.transparence {
                Some(transp_color) => {
                    let transp_color = rgb_to_argb(transp_color);
                    for y in v_start..v_end {
                        tx = tx_start;
                        for x in h_start..h_end {
                            posbuf = (((y * viewport_data.image_width as i32) + x) * 3) as usize;
                            let index_text = ((ty as usize) * text.largeur_usize + tx as usize);
                            
                            let col_text = *text_data.get_unchecked(index_text);
                            let col_text_rgb = argb_to_rgb(col_text);
                            if col_text != transp_color {
                                *framebuf.get_unchecked_mut(posbuf) = rgb_to_argb((
                                    (col_text_rgb.0 as f32 * collux_float.0).to_int_unchecked::<u8>(),
                                    (col_text_rgb.1 as f32 * collux_float.1).to_int_unchecked::<u8>(),
                                    (col_text_rgb.2 as f32 * collux_float.2).to_int_unchecked::<u8>()
                                ));
                                *zbuf.get_unchecked_mut(posbuf) = depth;
                                /*
                                image.replace_if_correct(posbuf, 
                                    (
                                        (col_text.0 as f32 * collux_float.0).to_int_unchecked::<u8>(),
                                        (col_text.1 as f32 * collux_float.1).to_int_unchecked::<u8>(),
                                        (col_text.2 as f32 * collux_float.2).to_int_unchecked::<u8>()
                                    ), depth,normal, framebuf);netstat -an | grep ESTABLISHED | wc -l
                                */
                            }
                            tx += dtx;
                        }
                        
                        ty += dty;
                    }
                },
                None => {
                    for y in v_start..v_end {
                        tx = tx_start;
                        for x in h_start..h_end {
                            posbuf = (((y * viewport_data.image_width as i32) + x) * 3) as usize;
                            let index_text = ((ty as usize) * text.largeur_usize + tx as usize);
                            *framebuf.get_unchecked_mut(posbuf) = *text_data.get_unchecked(index_text);

                            *zbuf.get_unchecked_mut(posbuf) = depth;
                            tx += dtx;
                        }
                        
                        ty += dty;
                    }
                }
            }
        }
                        
    }
    
}
