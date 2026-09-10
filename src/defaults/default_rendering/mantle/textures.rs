// idea :
// single large RGBA texture, "atlas"
// need to allocate space in 2D
// need to be able to convert that to texture buffer
// need to have mapping from texture name -> texture coordinates
// map UV using that as well
// how to handle texture bleeding ?
// - have wrapping in the shader if possible, and add a band of same texture around the texture, so allocate (dimension + margin)
// how to handle adding a new texture at runtime ?
// - easiest way is to pre-allocate all space required for textures
// - this works for low-res textures, allocating even 100 MB is perfectly fine
// - let's do that hehehe
// how to allocate ?
// - texture loading request contains all texture data as well as width/height
// - simplest reasonable algo :
//      - keep a record of latest texture
//      - put it on the right side of that, if no space, start of line and go down until it fits

use std::{collections::HashMap, sync::Arc};

use image::{ImageBuffer, Rgba};
use vulkano::{buffer::{Buffer, BufferCreateInfo, BufferUsage}, command_buffer::{AutoCommandBufferBuilder, BufferImageCopy, CopyBufferToImageInfo}, device::Device, format::Format, image::{Image, ImageCreateInfo, ImageType, ImageUsage}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}};

use crate::defaults::default_rendering::mantle::{api::MantleRequest, meshes::MemoryAllocator};

pub struct TextureAtlas {
    width:usize,
    height:usize,
    latest_texture:Option<String>,
    position_mapper:HashMap<String, AtlasPosition>,
    bleeding_margin:usize,
    image:Arc<Image>,
    allocator:MemoryAllocator,
    changed_textures:bool
}

impl TextureAtlas {
    pub fn new(width:usize, height:usize, bleeding_margin:usize, allocator:MemoryAllocator) -> Self {
        let image = Image::new(
            
            allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width as u32, height as u32, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::TRANSFER_SRC | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        ).unwrap();
        Self {
            width, 
            height,
            latest_texture: None,
            position_mapper: HashMap::with_capacity(256),
            bleeding_margin,
            image,
            allocator,
            changed_textures:false
        }
    }
    pub fn add_or_update_texture(&mut self, texture_name:String, texture_data:Vec<u8>, t_width:usize, t_height:usize, builder:&mut AutoCommandBufferBuilder<vulkano::command_buffer::PrimaryAutoCommandBuffer>) -> AtlasPosition {
        self.changed_textures = true;
        let pos = match self.position_mapper.get(&texture_name) {
            Some(texture) => {
                // TODO : update texture in this branch
                texture.clone()
            },
            None => {
                let (x,y) = self.get_next_texture_position(t_width, t_height);
                let pos = AtlasPosition { width: t_width, height: t_height, x, y, u_origin: (x as f32)/(self.width as f32), v_origin: (y as f32)/(self.height as f32) };
                self.position_mapper.insert(texture_name.clone(), pos.clone());
                self.latest_texture = Some(texture_name);
                pos
            }
        };
        self.updage_image_at(pos.clone(), texture_data, builder);
        pos
    }
    fn updage_image_at(&mut self, atlas_pos:AtlasPosition, texture_data:Vec<u8>, mut builder:&mut AutoCommandBufferBuilder<vulkano::command_buffer::PrimaryAutoCommandBuffer>) {
        let buffer = Buffer::from_iter(
            self.allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                // MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE
                memory_type_filter: MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            texture_data,
        )
        .unwrap();
        let mut copy_info = CopyBufferToImageInfo::buffer_image(buffer, self.image.clone());
        copy_info.regions = smallvec::smallvec![BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                
                image_subresource: self.image.subresource_layers(),
                
                image_offset: [atlas_pos.x as u32, atlas_pos.y as u32, 0],
                
                image_extent: [atlas_pos.width as u32, atlas_pos.height as u32, 1],
                
                ..Default::default()
            }];
        builder.copy_buffer_to_image(
            copy_info,
        ).unwrap();

    }
    pub fn must_barrier_out(&mut self) -> bool {
        let out = self.changed_textures;
        self.changed_textures = false;
        out
    }
    pub fn get_uv_for(&self, texture_name:String, u:f32, v:f32) -> [f32 ; 2] {
        let pos = self.position_mapper.get(&texture_name).expect("Texture was supposed to be added before it was queried in the atlas");
        [
            u * pos.get_u_factor(self.width) + pos.u_origin,
            v * pos.get_v_factor(self.height) + pos.v_origin
        ]
    }
    pub fn get_image(&self) -> Arc<Image> {
        self.image.clone()
    }
    fn get_next_texture_position(&self, width:usize, height:usize) -> (usize, usize) {
        match &self.latest_texture {
            Some(text) => {
                let atlas_pos = self.position_mapper.get(text).unwrap();
                
                let conflict = self.is_intersecting_with_any_texture(atlas_pos.x + atlas_pos.width + self.bleeding_margin * 2, atlas_pos.y, width, height);

                if conflict.is_none() && self.is_in_atlas(atlas_pos.x + atlas_pos.width + self.bleeding_margin * 2, atlas_pos.y, width, height) {
                    (atlas_pos.x + atlas_pos.width + self.bleeding_margin * 2, atlas_pos.y)
                }
                else {
                    let mut candidate = (self.bleeding_margin, self.bleeding_margin);
                    loop {
                        let mut x_offset = 0;
                        while let Some(conflict) = self.is_intersecting_with_any_texture(candidate.0, candidate.1, width, height) {
                            candidate.1 += conflict.height + self.bleeding_margin;
                            x_offset = x_offset.max(conflict.x + conflict.width + self.bleeding_margin);
                        }
                        if self.is_in_atlas(candidate.0, candidate.1, width, height) {
                            break
                        }
                        else {
                            candidate.0 += x_offset;
                        }
                    }
                    candidate

                }

            },
            None => (self.bleeding_margin, self.bleeding_margin)
        }
    }
    fn is_in_atlas(&self, x:usize, y:usize, width:usize, height:usize) -> bool {
        x + width + self.bleeding_margin < self.width && y + height + self.bleeding_margin < self.height
    }
    fn is_intersecting_with_any_texture(&self, x:usize, y:usize, width:usize, height:usize) -> Option<AtlasPosition> {
        
        let x1 = x - self.bleeding_margin;
        let y1 = y - self.bleeding_margin;
        let x2 = x + width + self.bleeding_margin;
        let y2 = y + height + self.bleeding_margin;

        for pos in self.position_mapper.values() {
            let ((px1, py1), (px2, py2)) = pos.get_rectangle_points(self.bleeding_margin);
            if (x2 >= px1 && x1 <= px2)
            && (y2 >= py1 && y1 <= py2) {
                return Some(pos.clone())
            }
        }
        None
    }
}


#[derive(Clone)]
pub struct AtlasPosition {
    width:usize,
    height:usize,
    x:usize,
    y:usize,
    u_origin:f32,
    v_origin:f32,
}

impl AtlasPosition {
    pub fn get_u_factor(&self, atlas_width:usize) -> f32 {
        (self.width as f32)/(atlas_width as f32)
    }
    pub fn get_v_factor(&self, atlas_height:usize) -> f32 {
        (self.height as f32)/(atlas_height as f32)
    }
    pub fn get_rectangle_points(&self, bleeding_margin:usize) -> ((usize, usize), (usize, usize)) {

        let x1 = self.x - bleeding_margin;
        let y1 = self.y - bleeding_margin;
        let x2 = self.x + self.width + bleeding_margin;
        let y2 = self.y + self.height + bleeding_margin;
        (
            (x1, y1),
            (x2, y2)
        )
    }
}

pub fn load_single_texture(path: &str) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ()> {
    // tries to load an image in the "textures" folder
    match image::io::Reader::open(path) {
        Ok(texture) => {
            //dbg!(nom);
            let image_buffer = texture.decode().unwrap().to_rgba8();
            return Ok(image_buffer);
        }
        Err(err) => {
            println!(
                "texture : {} couldn't be loaded, error {}",
                path, err
            );
            return Err(());
        }
    }
}

pub fn buffer_to_request(name:String, buffer:ImageBuffer<Rgba<u8>, Vec<u8>>) -> MantleRequest {

    let out = buffer.clone().into_raw();
    MantleRequest::CreateOrUpdateTexture { name, texture_data: out, width: buffer.width() as usize, height:  buffer.height() as usize }
}