use std::os::raw;
use std::simd::Simd;
use std::sync::RwLock;

use image::io::Reader as ImageReader;
use image::ImageBuffer;
use image::Rgb;
#[derive(Debug)]
pub struct MipMaps {
    maps:Vec<MipMap>
}

#[derive(Debug)]
pub struct MipMap {
    pub data:Vec<u32>,
    pub f_data:Vec<Simd<f32, 4>>,
    pub largeur_usize:usize,
    pub hauteur:f32,
    pub largeur:f32,
    pub len_m1:usize,
    pub len_f32:f32,
}

fn is_in(x:isize, y:isize, largeur:isize, hauteur:isize) -> bool {
    x >= 0 && y >= 0 && x < largeur && y < hauteur
}

fn vec_rgb_to_vec_argb(vector:&Vec<(u8,u8,u8)>) -> Vec<u32> {
    let mut new_vec = Vec::with_capacity(vector.len());
    for rgb in vector {
        new_vec.push(rgb_to_argb(*rgb));
    }
    new_vec
}

pub const fn rgb_to_argb((r,g,b):(u8,u8,u8)) -> u32 {
    ((r as u32) << 16) + ((g as u32) << 8) + ((b as u32)) 
}

pub fn argb_to_rgb(argb:u32) -> (u8,u8,u8) {
    (
        (argb >> 16 & 255_u32) as u8,
        (argb >> 8 & 255_u32) as u8,
        (argb & 255_u32) as u8,
    )
}

pub fn rgbu_to_rgbf((r,g,b):(u8,u8,u8)) -> (f32, f32, f32) {
    (
        r as f32/255.0,
        g as f32/255.0,
        b as f32/255.0,
    )
}

fn get_average_color_around(x:isize, y:isize, largeur:isize, hauteur:isize, data:&Vec<u32>) -> (u8, u8, u8) {
    const TESTS:[(isize, isize) ; 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];
    let (mut r, mut g, mut b) = (0,0,0);
    let mut cool_number = 0;
    for (dx, dy) in TESTS {
        if is_in(x + dx, y + dy, largeur, hauteur) {
            let rgb = argb_to_rgb(data[(y + dy) as usize * largeur as usize + (x + dx) as usize]);
            r += rgb.0 as i32;
            g += rgb.1 as i32;
            b += rgb.2 as i32;
            cool_number += 1;
        }
    }
    if cool_number == 0 {
        for (dx, dy) in TESTS {
            println!("{} {} {} {}", x + dx, y + dy, largeur, hauteur);
        }
    }
    r /= cool_number;
    g /= cool_number;
    b /= cool_number;
    (r as u8, g as u8, b as u8)
}

fn get_averaged_down_data(data:&Vec<u32>, largeur:isize, hauteur:isize) -> (Vec<u32>, Vec<Simd<f32, 4>>, isize, isize) {
    let small_largeur = largeur/2;
    let small_hauteur = hauteur/2;
    let (mut new_data, mut new_data_f) = (Vec::with_capacity(small_largeur as usize * small_hauteur as usize), Vec::with_capacity(small_largeur as usize * small_hauteur as usize));
    for y in 0..small_hauteur {
        for x in 0..small_largeur {
            let rgb = get_average_color_around(x*2, y*2, largeur, hauteur, data);
            new_data.push(rgb_to_argb(rgb));
            new_data_f.push(Simd::from_array([rgb.0 as f32, rgb.1 as f32, rgb.2 as f32 , 0.0]));
        }
    }
    (new_data, new_data_f, small_largeur, small_hauteur)
}

impl MipMaps {
    pub fn new(data:Vec<(u8,u8,u8)>, f_data:Vec<Simd<f32,4>>, mut largeur:isize, mut hauteur:isize) -> Self {
        let mut mipmaps = vec![MipMap {len_m1:data.len() - 1, len_f32:data.len() as f32, data:vec_rgb_to_vec_argb(&data), f_data, largeur_usize:largeur as usize, hauteur:hauteur as f32, largeur:largeur as f32}];
        let mut data = mipmaps[0].data.clone();
        loop {
            let (new_data, new_data_f, small_largeur, small_hauteur) = get_averaged_down_data(&data, largeur, hauteur);
            if small_hauteur > 1 && small_largeur > 1 {
                mipmaps.push(MipMap { len_m1:new_data.len() - 1, len_f32:new_data.len() as f32,data:new_data, f_data:new_data_f, largeur_usize:small_largeur as usize, hauteur:small_hauteur as f32, largeur: small_largeur as f32 });
                hauteur = small_hauteur;
                largeur = small_largeur;
                data = mipmaps.last().unwrap().data.clone();
            }
            else {
                break;
            }
        }
        Self { maps: mipmaps }
    }
}

#[derive(Debug)]
pub struct DataTexture {
    pub mip_maps:MipMaps,
    pub raw_data : RwLock<Vec<u8>>,
    pub transparence: Option<(u8, u8, u8)>,
    pub nom: String,
    pub average: (u8,u8,u8)
}

impl DataTexture {
    pub fn get_mip_map(&self, id:usize) -> &MipMap {
        &self.mip_maps.maps[id.clamp(0, self.mip_maps.maps.len() - 1)]
    }
}

#[derive(Clone)]
pub enum TextureSetID {
    ID(usize),
    Name(String)
}
impl TextureSetID {
    pub fn convert(&self, textures:&Textures) -> Option<Self> {
        match &self {
            TextureSetID::ID(id) => None,
            TextureSetID::Name(name) => textures.get_id_with_name(name)
        }
    }
}

#[derive(Debug, Clone)]
struct TextureId {
    id: usize,
    deltatick: u16,
}
#[derive(Debug, Clone)]
pub struct TextureSet {
    set: Vec<TextureId>,
    tick: u16,
    act: usize,
    name:String,
}

impl TextureSet {
    fn do_tick(&mut self) {
        self.tick += 1;
        if self.tick == self.set[self.act].deltatick {
            self.tick = 0;
            self.forward_act();
        }
    }
    fn forward_act(&mut self) {
        if self.act == self.set.len() - 1 {
            self.act = 0;
        } else {
            self.act += 1;
        }
    }
}

pub struct Textures {
    textures: Vec<DataTexture>,
    sets: Vec<TextureSet>,
}

fn charge_image(nom: &str) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, ()> {
    // tries to load an image in the "textures" folder
    let chemin = format!("textures/{}", nom);
    match ImageReader::open(chemin.as_str()) {
        Ok(texture) => {
            //dbg!(nom);
            let image_buffer = texture.decode().unwrap().to_rgb8();
            return Ok(image_buffer);
        }
        Err(err) => {
            println!(
                "texture : {} introuvable dans le dossier cartes... erreur {}",
                nom, err
            );
            return Err(());
        }
    }
}

fn charge_image_sur(nom: &str) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    // if it can't load the given image, load the default "arbre.png" image
    match charge_image(nom) {
        Ok(image_buffer) => return image_buffer,
        Err(()) => return charge_image("arbre.png").unwrap(),
    }
}

fn convert_image_buffer_into_vec_u8(buffer: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Vec<(u8, u8, u8)> {
    let mut out = Vec::new();
    for y in 0..buffer.height() {
        for x in 0..buffer.width() {
            let col = buffer.get_pixel(x, y);
            out.push((col.0[0], col.0[1], col.0[2]));
        }
    }
    out
}

impl Textures {
    pub fn clear_all(&mut self) {
        self.sets.clear();
        self.textures.clear();
    }
    pub fn get_all_sets(&self) -> &Vec<TextureSet> {
        &self.sets
    }
    pub fn get_all_datas(&self) -> &Vec<DataTexture> {
        &self.textures
    }
    pub fn get_text_with_id(&self, id: usize) -> &DataTexture {
        let set = &self.sets[id];
        &self.textures[set.set[set.act].id]
    }
    pub fn get_data_with_id(&self, id:&TextureSetID) -> &DataTexture {
        match id {
            TextureSetID::ID(ident) => self.get_text_with_id(*ident),
            TextureSetID::Name(name) => match self.get_id_with_name(name) {
                Some(good_id) => match good_id { TextureSetID::ID(ident) => self.get_text_with_id(ident), TextureSetID::Name(_) => panic!("couldn't get ident")}
                None => panic!("couldn't get ident, didn't find texture")
            }
        }
    }
    pub fn get_id_with_name(&self, name:&String) -> Option<TextureSetID> {
        self.sets.iter().enumerate().find(|(i,set)| {&set.name == name}).map(|(i, _)| {TextureSetID::ID(i)})
    }
    pub fn new() -> Textures {
        Textures {
            textures: Vec::new(),
            sets: Vec::new(),
        }
    }
    pub fn add_set(
        &mut self,
        name: String,
        name_of_set:String,
        deltatick: u16,
        transparence: Option<(u8, u8, u8)>,
    ) -> usize {
        self.sets.push(TextureSet {
            act: 0,
            tick: 0,
            set: Vec::new(),
            name:name_of_set
        });
        self.add_texture_to_set(self.sets.len() - 1, name, deltatick, transparence);
        self.sets.len() - 1
    }
    fn exists(&self, name: String) -> Option<usize> {
        for i in 0..self.textures.len() {
            if self.textures[i].nom == name {
                return Some(i);
            }
        }
        None
    }
    pub fn add_set_raw(&mut self, set:TextureSet) {
        self.sets.push(set);
    }
    pub fn load_new_texture(&mut self, name: String, transparence: Option<(u8, u8, u8)>) -> usize {
        let data = charge_image_sur(name.trim());
        let largeur_image_usize = data.width() as usize;
        let hauteur_image = data.height() as f32;
        let largeur_image = data.width() as f32;
        //dbg!(hauteur_image, largeur_image);
        let data = convert_image_buffer_into_vec_u8(data);
        let mut raw_data = Vec::with_capacity(data.len());
        for col in &data {
            raw_data.push(col.0);
            raw_data.push(col.1);
            raw_data.push(col.2);
        }

        let mut average = (0,0,0);
        let mut counted = 0;
        for col in data.iter() {
            average.0 += col.0 as u32;
            average.1 += col.1 as u32;
            average.2 += col.2 as u32;
            counted += 1;
        }
        average.0 /= counted;
        average.1 /= counted;
        average.2 /= counted;
        let mut f_data = Vec::with_capacity(data.len());
        for col in &data {
            f_data.push(Simd::from_array([col.0 as f32, col.1 as f32, col.2 as f32, 0.0]))
        }
        let mip_maps = MipMaps::new(data, f_data, largeur_image as isize, hauteur_image as isize);
        self.textures.push(DataTexture {
            mip_maps,
            transparence,
            nom: name,
            raw_data: RwLock::new(raw_data),
            average: (average.0 as u8, average.1 as u8, average.2 as u8)
        });

        self.textures.len() - 1
    }
    pub fn add_texture_to_set(
        &mut self,
        set_id: usize,
        name: String,
        deltatick: u16,
        transparence: Option<(u8, u8, u8)>,
    ) {
        match self.exists(name.clone()) {
            Some(id) => {
                self.sets[set_id].set.push(TextureId { id, deltatick });
            }
            None => {
                let id = self.load_new_texture(name, transparence);
                self.sets[set_id].set.push(TextureId { id, deltatick });
            }
        }
    }
    pub fn add_textures_to_set(
        &mut self,
        set_id: usize,
        textures: Vec<(String, u16, Option<(u8, u8, u8)>)>,
    ) {
        for (name, deltatick, transparence) in textures {
            self.add_texture_to_set(set_id, name, deltatick, transparence);
        }
    }
    pub fn add_set_with_many_textures(
        &mut self,
        name_of_set:String,
        textures: Vec<(String, u16, Option<(u8, u8, u8)>)>,
    ) -> usize {
        self.sets.push(TextureSet {
            act: 0,
            tick: 0,
            set: Vec::new(),
            name:name_of_set
        });
        for (name, deltatick, transparence) in textures {
            self.add_texture_to_set(self.sets.len() - 1, name, deltatick, transparence);
        }
        //dbg!(&self.sets);
        self.sets.len() - 1
    }
    pub fn tick_all_sets(&mut self) {
        for set in &mut self.sets {
            set.do_tick();
        }
    }
    pub fn add_generated_texture_set(&mut self, name:String, raw_argb_data:Vec<u32>, width:usize, height:usize) {
        let mut into_rgb = Vec::with_capacity(raw_argb_data.len() * 3);
        let mut rgb_tuples = Vec::with_capacity(raw_argb_data.len());
        let mut float_simd_vecs = Vec::with_capacity(raw_argb_data.len());
        let mut avg_r = 0;
        let mut avg_g = 0;
        let mut avg_b = 0;
        for argb in &raw_argb_data {
            let (r,g,b) = argb_to_rgb(*argb);
            into_rgb.push(r);
            into_rgb.push(g);
            into_rgb.push(b);
            avg_r += r as u32;
            avg_g += g as u32;
            avg_b += b as u32;
            rgb_tuples.push((r,g,b));
            float_simd_vecs.push(Simd::from_array([r as f32, g as f32, b as f32, 0.0]));
        }
        avg_r /= raw_argb_data.len() as u32;
        avg_g /= raw_argb_data.len() as u32;
        avg_b /= raw_argb_data.len() as u32;
        self.textures.push(DataTexture { mip_maps: MipMaps::new(rgb_tuples, float_simd_vecs, width as isize, height as isize), raw_data:RwLock::new(into_rgb), transparence: None, nom: name.clone(), average: (avg_r as u8, avg_g as u8, avg_b as u8) });
        self.add_set_raw(TextureSet { set: vec![TextureId{id:self.textures.len() - 1, deltatick:0}], tick: 0, act: 0, name });
    }

    pub fn add_generated_texture_multiset(&mut self, name: String, raw_argb_datas:Vec<Vec<u32>>, width:usize, height:usize, deltatick: u16, transparence: Option<(u8, u8, u8)>) {
        let mut ids = Vec::with_capacity(raw_argb_datas.len());
        for i in 0..raw_argb_datas.len() {
            let raw_argb_data = &raw_argb_datas[i];
            let mut into_rgb = Vec::with_capacity(raw_argb_data.len() * 3);
            let mut rgb_tuples = Vec::with_capacity(raw_argb_data.len());
            let mut float_simd_vecs = Vec::with_capacity(raw_argb_data.len());
            let mut avg_r = 0;
            let mut avg_g = 0;
            let mut avg_b = 0;
            for argb in raw_argb_data {
                let (r,g,b) = argb_to_rgb(*argb);
                into_rgb.push(r);
                into_rgb.push(g);
                into_rgb.push(b);
                avg_r += r as u32;
                avg_g += g as u32;
                avg_b += b as u32;
                rgb_tuples.push((r,g,b));
                float_simd_vecs.push(Simd::from_array([r as f32, g as f32, b as f32, 0.0]));
            }
            avg_r /= raw_argb_data.len() as u32;
            avg_g /= raw_argb_data.len() as u32;
            avg_b /= raw_argb_data.len() as u32;
            ids.push(TextureId{id:self.textures.len(), deltatick});
            self.textures.push(DataTexture { mip_maps: MipMaps::new(rgb_tuples, float_simd_vecs, width as isize, height as isize), raw_data:RwLock::new(into_rgb), transparence, nom: name.clone(), average: (avg_r as u8, avg_g as u8, avg_b as u8) });
            
        }
        self.add_set_raw(TextureSet { set: ids, tick: 0, act: 0, name });
    
    }
}
