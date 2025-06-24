use std::{cell::SyncUnsafeCell, simd::{num::SimdFloat, Simd}};

use crate::horde::{frontend::HordeWindowDimensions, utils::{late_alloc_mpmc_vec::LAMPMCVec, parallel_counter::ParallelCounter}};

use super::{simd_geo::LANE_COUNT, triangles::SingleFullTriangle};

#[derive(Clone)]
pub struct PreCalcdData {
    pub area:f32,
    pub inv_area:f32,
    pub bounding_box_pre_binned:((i32, i32), (i32, i32)),
    pub mip_map:usize,
    pub packed_normal:u32

}

impl PreCalcdData {
    pub fn new(area:f32, bounding_box_pre_binned:((i32,i32), (i32,i32)), mip_map:usize, packed_normal:u32) -> Self {
        Self { area, inv_area:1.0/area, bounding_box_pre_binned, mip_map, packed_normal }
    }
}

pub struct Bins {
    triangles:LAMPMCVec<(SingleFullTriangle, PreCalcdData)>,
    bins:Vec<Bin>,
    clear_buf:Vec<u32>,
    bins_counter:ParallelCounter,
    real_bin_size:usize,
    inv_real_bin_size_f:f32,
    bin_size:usize,
    horizontal_bins:usize,
    dims:HordeWindowDimensions
}

impl Bins {
    pub fn new(dimensions:HordeWindowDimensions, bin_size:usize) -> Self {
        if bin_size == 0 {
            panic!("Did you just try to use a bin size of 0 ? do better.")
        }
        let real_bin_size = LANE_COUNT * bin_size;
        let mut horizontal = (dimensions.get_width() / real_bin_size) + if dimensions.get_width() % real_bin_size == 0 {0} else {1};
        let mut vertical = (dimensions.get_height() / real_bin_size) + if dimensions.get_height() % real_bin_size == 0 {0} else {1};
        let mut bins = Vec::with_capacity(horizontal * vertical);
        for y in 0..vertical {
            for x in 0..horizontal {
                bins.push(Bin {
                    image_data:SyncUnsafeCell::new(InternalBinImageData::from_bin_size(real_bin_size)),
                    triangle_ids:LAMPMCVec::new(2000),
                    start_x:x * real_bin_size,
                    start_y:y * real_bin_size,
                    end_x:(x + 1) * real_bin_size,
                    end_y:(y + 1) * real_bin_size,
                    start_x_i:(x * real_bin_size) as i32,
                    start_y_i:(y * real_bin_size) as i32,
                    start_x_f:(x * real_bin_size) as f32,
                    start_y_f:(y * real_bin_size) as f32,
                    end_x_f:((x + 1) * real_bin_size) as f32,
                    end_y_f:((y + 1) * real_bin_size) as f32,
                    end_x_i:((x + 1) * real_bin_size) as i32,
                    end_y_i:((y + 1) * real_bin_size) as i32,

                });
            }
        }
        Bins {clear_buf:vec![0 ; real_bin_size * real_bin_size],inv_real_bin_size_f:1.0/(real_bin_size as f32), triangles: LAMPMCVec::new(1024), bins, bins_counter:ParallelCounter::new(horizontal * vertical, 1), real_bin_size, dims:dimensions, horizontal_bins:horizontal, bin_size }
    }
    pub fn drop_triangles(&self) {
        unsafe {
            dbg!(self.triangles.len());
            self.triangles.consume_all_elems(&mut |elem| {});
        }
    }
    pub fn get_clear(&self) -> &Vec<u32> {
        &self.clear_buf
    }
    pub fn push_triangle(&self, tri:SingleFullTriangle,mut pre_calcd_data:PreCalcdData, pre_bounding_box_f:[f32 ; 4]) {

        unsafe {    
        pre_calcd_data.bounding_box_pre_binned = ((pre_bounding_box_f[0] as i32, pre_bounding_box_f[1] as i32), (pre_bounding_box_f[2] as i32, pre_bounding_box_f[3] as i32));
        if pre_calcd_data.bounding_box_pre_binned.1.1 - pre_calcd_data.bounding_box_pre_binned.0.1 > 0 {
            let triangle_id = self.triangles.push((tri, pre_calcd_data));
            match triangle_id {
                Ok(triangle_id) => {
                    let divided_simd = (Simd::from_array(pre_bounding_box_f) * Simd::splat(self.inv_real_bin_size_f)).to_int_unchecked::<usize>().to_array();
                    let divided = ((divided_simd[0], divided_simd[1]), (divided_simd[2], divided_simd[3]));
                    let mut position = 0;//divided.0.1 * self.horizontal_bins + divided.0.0;
                    'o : for y in divided.0.1..=divided.1.1 {

                        position = y * self.horizontal_bins + divided.0.0;
                        'i: for x in divided.0.0..=divided.1.0 {
                            if position < self.bins.len() {
                                self.bins.get_unchecked(position).triangle_ids.push(triangle_id);
                            }
                            else {
                                break 'i;
                            }
                            position += 1;
                        }
                    }
                },
                Err(()) => ()
            }
        }
        
        // dbg!(start_x, start_y, end_x, end_y);
        //dbg!(triangle_id);
            
        }
        
    }
    pub fn reset_internal_counter(&self) {
        self.bins_counter.reset();
    }
    pub fn do_for_all_triangle_ids<F:FnMut(&(SingleFullTriangle, PreCalcdData), &Bin), G:FnMut(&Bin)>(&self, func:&mut F, post_func:&mut G) {
        let mut counter = self.bins_counter.clone();
        counter.initialise();
        for i in counter {

            unsafe {
                self.bins[i].triangle_ids.consume_all_elems(&mut |id| {
                        func(self.triangles.get_unchecked(*id), &self.bins[i])
                    }
                    
                );
            }
            post_func(&self.bins[i])
        }
    }
}

pub struct Bin {
    triangle_ids:LAMPMCVec<usize>,
    pub image_data:SyncUnsafeCell<InternalBinImageData>,
    start_x:usize,
    start_y:usize,
    end_x:usize,
    end_y:usize,
    pub start_x_i:i32,
    pub start_y_i:i32,
    pub start_x_f:f32,
    pub start_y_f:f32,
    pub end_x_f:f32,
    pub end_y_f:f32,
    pub end_x_i:i32,
    pub end_y_i:i32
}


pub struct InternalBinImageData {
    pub frambuf:Vec<u32>,
    pub zbuf:Vec<f32>,
    pub nbuf:Vec<u32>,
    pub bin_size:usize,
    pub bin_size_i:i32,
}

impl InternalBinImageData {
    pub fn from_bin_size(bin_size:usize) -> Self {
        Self { frambuf: vec![0; bin_size * bin_size], zbuf: vec![0.0; bin_size * bin_size], nbuf: vec![0; bin_size * bin_size], bin_size, bin_size_i: bin_size as i32 }
    }
    pub fn clear_bufs(&mut self, clear_vec:&Vec<u32>) {
        self.frambuf.copy_from_slice(clear_vec);
        self.nbuf.copy_from_slice(clear_vec); 
        // unsafe {self.zbuf.copy_from_slice(std::mem::transmute::<&Vec<u32>, &Vec<f32>>(clear_vec))};
    }
}