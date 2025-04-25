use std::{collections::{HashMap, HashSet}, hash::{Hash, RandomState}, path::PathBuf, sync::{atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU8, Ordering}, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use cosmic_text::{fontdb::ID, Align, Attrs, Buffer, CacheKeyFlags, Color, Family, FontSystem, Metrics, SwashCache};
use crossbeam::channel::{unbounded, Receiver, Sender};
use image::{io::Reader, ImageBuffer, Rgb};

use crate::{defaults::default_rendering::{vectorinator::textures::{argb_to_rgb, rgb_to_argb, rgbu_to_rgbf}, vectorinator_binned::textures::{MipMap, TextureSetID, Textures}}, horde::{frontend::{MouseState, SyncUnsafeHordeFramebuffer}, scheduler::IndividualTask}};

// what's left to do :
// - multiple types of positioning (at least to the left/centered)
// - support scrolling

#[derive(Clone)]
pub struct SimpleUI<UE:UserEvent> {
    elements:Arc<RwLock<Vec<UIElement<UE>>>>,
    images:Arc<RwLock<UIImages>>,
    framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>,
    user_events:Sender<UE>,
    full_updates:Receiver<(usize, usize)>,
    mouse_pos:MouseState,
    font_system:Arc<RwLock<FontSystem>>,
    cache:Arc<RwLock<SwashCache>>
}

impl<UE:UserEvent> SimpleUI<UE> {
    pub fn new(expected_elements:usize, expected_images:usize, framebuf:Arc<RwLock<SyncUnsafeHordeFramebuffer>>, mouse_pos:MouseState, full_updates:Receiver<(usize, usize)>) -> (Self, Receiver<UE>) {
        let (sender, receiver) = unbounded();
        (
            Self {
                elements: Arc::new(RwLock::new(Vec::with_capacity(expected_elements))),
                images: Arc::new(RwLock::new(UIImages { images: HashMap::with_capacity(expected_images) })),
                framebuf,
                user_events:sender,
                full_updates,
                mouse_pos,
                font_system:Arc::new(RwLock::new(FontSystem::new())),
                cache:Arc::new(RwLock::new(SwashCache::new()))
            },
            receiver
        )
    }
    pub fn change_visibility_of(&self, id:UIElementID, visibility:bool) {
        let elements = self.elements.read().unwrap();
        let id = self.get_index_from_id(id, &elements);
        elements[id].visible.store(visibility, Ordering::Relaxed);
    }
    pub fn get_index_from_id(&self, id:UIElementID, elements:&Vec<UIElement<UE>>) -> usize {
        match id {
            UIElementID::Index(given) => given,
            UIElementID::Name(name) => {
                let mut index = None;
                for (i, element) in elements.iter().enumerate() {
                    if &element.name == &name {
                        index = Some(i);
                    }
                }
                if index.is_none() {
                    panic!("No element matching name {} found !", name.clone());
                }
                index.unwrap()
            }
        }
    }
    pub fn change_content_of(&self,id:UIElementID, index:usize, content:UIElementContent) {
        let mut elements = self.elements.write().unwrap();
        let id = self.get_index_from_id(id, &elements);
        elements[id].cache.write().unwrap().clear();
        elements[id].content[index] = content;
    }
    pub fn change_content_background_of(&self, id:UIElementID, index:usize, background:UIElementBackground ) {
        let mut elements = self.elements.write().unwrap();
        let id = self.get_index_from_id(id, &elements);
        elements[id].cache.write().unwrap().clear();
        elements[id].content_background[index] = background;
    }
}

impl<UE:UserEvent> IndividualTask for SimpleUI<UE> {
    type TD = usize;
    type TID = usize;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize) {
        match task_id {
            0 => { //Sequential
                let mut read = self.get_read();
                read.do_everything_read();
            },
            1 => { //Sequential
                let mut write = self.get_write();
                write.draw_all();
            },
            _ => panic!("No task ID after 1 for SimpleUI"),
        }
    }
}

pub struct SimpleUIRead<'a, UE:UserEvent> {
    elements:RwLockReadGuard<'a, Vec<UIElement<UE>>>,
    images:RwLockReadGuard<'a, UIImages>,
    framebuf:RwLockReadGuard<'a, SyncUnsafeHordeFramebuffer>,
    font_system:RwLockWriteGuard<'a, FontSystem>,
    cache:RwLockWriteGuard<'a, SwashCache>,
    user_events:Sender<UE>,
    full_updates:Receiver<(usize, usize)>,
    mouse_pos:MouseState
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntegerUIVector {
    x:i32,
    y:i32
}

impl<'a, UE:UserEvent> SimpleUIRead<'a, UE> {
    pub fn get_integer_dimensions_of(&self, start_elt:usize) -> IntegerUIVector {
        let start = &self.elements[start_elt];
        match &start.dimensions {
            UIDimensions::Decided(vector) => IntegerUIVector {
                x: match vector.x {
                    UIUnit::RelativeToParentOrigin(value) => value,
                    UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32,
                    UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32,
                },
                y: match vector.y {
                    UIUnit::RelativeToParentOrigin(value) => value,
                    UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32,
                    UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32,
                },
            },
        }
    }
    pub fn get_content_integer_origins(&self, elt:&UIElementID) -> IntegerUIVector {
        let id = self.get_element_by_id(elt);
        let origin = self.get_integer_origin_of(id);
        let padding = self.get_integer_padding_of(elt);
        IntegerUIVector {
            x: padding.x + origin.x,
            y: padding.y + origin.y,
        }
    }
    pub fn get_content_integer_dimensions(&self, elt:&UIElementID) -> IntegerUIVector {
        let id = self.get_element_by_id(elt);
        let padding = self.get_integer_padding_of(elt);
        let dimensions = self.get_integer_dimensions_of(id);
        //dbg!(padding.clone(), dimensions.clone());
        IntegerUIVector {
            x: dimensions.x - padding.x * 2,
            y: dimensions.y - padding.y * 2,
        }
    }
    pub fn get_integer_padding_of(&self, elt:&UIElementID) -> IntegerUIVector {
        let id = self.get_element_by_id(elt);
        let element = &self.elements[id];
        IntegerUIVector {
            x: match element.padding_to_content.x {
                UIUnit::RelativeToParentOrigin(value) => value,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of(id).y as f32 * prop) as i32,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of(id).x as f32 * prop) as i32,
            },
            y: match element.padding_to_content.y {
                UIUnit::RelativeToParentOrigin(value) => value,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of(id).y as f32 * prop) as i32,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of(id).x as f32 * prop) as i32,
            },
        }
    }
    pub fn get_integer_origin_of(&self, start_elt:usize) -> IntegerUIVector {
        let start = &self.elements[start_elt];
        IntegerUIVector {
            x: match start.origin.x {
                UIUnit::RelativeToParentOrigin(value) => value + self.get_integer_origin_of_parent(start.parent.clone()).x,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).x,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).x,
            },
            y: match start.origin.y {
                UIUnit::RelativeToParentOrigin(value) => value + self.get_integer_origin_of_parent(start.parent.clone()).y,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).y,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).y,
            },
        }
    }
    pub fn get_integer_origin_of_parent(&self, parent:Option<UIElementID>) -> IntegerUIVector {
        match &parent {
            Some(id) => self.get_integer_origin_of(self.get_element_by_id(id)),
            None => IntegerUIVector { x: 0, y:0 }
        }
    }
    pub fn get_integer_dimensions_of_parent(&self, parent:Option<UIElementID>) -> IntegerUIVector {
        match &parent {
            Some(id) => self.get_integer_dimensions_of(self.get_element_by_id(id)),
            None => IntegerUIVector { x: self.framebuf.get_dims().get_width() as i32, y: self.framebuf.get_dims().get_height() as i32 }
        }
    }
    pub fn get_element_by_id(&self, id:&UIElementID) -> usize {
        match id {
            UIElementID::Index(given) => *given,
            UIElementID::Name(name) => {
                for (i, element) in self.elements.iter().enumerate() {
                    if &element.name == name {
                        return i
                    }
                }
                panic!("No element matching name {} found !", name.clone());
            }
        }
    }
    pub fn get_bounding_box(&self, id:&UIElementID) -> UIBoundingBox {
        let origin = self.get_integer_origin_of(self.get_element_by_id(id));
        let dimensions = self.get_integer_dimensions_of(self.get_element_by_id(id));
        UIBoundingBox::new(origin.x, origin.y, origin.x + dimensions.x, origin.y + dimensions.y)
    }
    pub fn get_affected_by_action(&self, action:UIUserAction, x:i32, y:i32) -> HashSet<usize, RandomState> {
        let mut ids = HashSet::with_capacity(2);
        for (i, element) in self.elements.iter().enumerate() {
            let bounding_box = self.get_bounding_box(&UIElementID::Index(i));
            if bounding_box.is_in(x, y) {
                for reac in &element.reactions {
                    if reac.0 == action {ids.insert(i); break}
                }
            }
        }
        ids
    }
    pub fn apply_full_updates(&self) {
        match &self.full_updates.try_recv() {
            Ok(event) => for elt in self.elements.iter() {
                let mut cache = elt.cache.write().unwrap();
                cache.clear();
            },
            Err(_) => ()
        }
    }
    pub fn get_triple_made_by(&self, action:UIUserAction, id:usize) -> (usize, usize, usize) {
        let mut background = 0;
        let mut content = 0;
        let mut content_background = 0;
        let element = &self.elements[id];
        for (react, event) in &element.reactions {
            if *react == action {
                match event {
                    UIEvent::ChangeContentBackground(value) => content_background = *value,
                    UIEvent::ChangeContent(value) => content = *value,
                    UIEvent::ChangeBackground(value) => content = *value,
                    UIEvent::User(evt) => {self.user_events.send(evt.clone());},
                }
            }
        }
        (background, content, content_background)
    }
    pub fn get_all_display_triples(&self, x:i32, y:i32) -> Vec<(usize, (usize, usize, usize))> {
        let mut triples = Vec::with_capacity(self.elements.len());
        let action = match self.mouse_pos.get_current_state().left {
            0 => UIUserAction::Nothing,
            1 => UIUserAction::Hovering,
            2 => UIUserAction::Clicking,
            _ => UIUserAction::Nothing
        };
        let actions = [UIUserAction::Clicking, UIUserAction::Hovering, UIUserAction::Nothing];
        let mut all_ids_set = HashSet::with_capacity(self.elements.len());
        for i in 0..self.elements.len() {
            all_ids_set.insert(i);
        }
        //for action in &actions {
            let affected_by = self.get_affected_by_action(action, x, y);
            
            for aff in &affected_by {
                let triple = self.get_triple_made_by(action, *aff);
                triples.push((*aff, triple));
                let element = &self.elements[*aff];
                element.chosen_triplet.0.store(triple.0 as isize, Ordering::Relaxed);
                element.chosen_triplet.1.store(triple.1 as isize, Ordering::Relaxed);
                element.chosen_triplet.2.store(triple.2 as isize, Ordering::Relaxed);
            }
            for not_aff in all_ids_set.difference(&affected_by) {
                let element = &self.elements[*not_aff];
                element.chosen_triplet.0.store(0, Ordering::Relaxed);
                element.chosen_triplet.1.store(0, Ordering::Relaxed);
                element.chosen_triplet.2.store(0, Ordering::Relaxed);
                triples.push((*not_aff, (0,0,0)));
            }
        //}
        triples
    }
    pub fn build_cache(&mut self, triples:Vec<(usize, (usize, usize, usize))>) {
        for (id, triple) in triples {

            let mut have_to_create_cache = false;

            {

                let element = &self.elements[id];
                let mut cache = element.cache.write().unwrap();
                match cache.get(&triple) {
                    Some(value) => (),
                    None => {
                        have_to_create_cache = true;
                        
                    }
                }
            }
            
            let mut new_element = if have_to_create_cache {
                let (dims, content_dims, background, content, content_background) = {

                    let element = &self.elements[id];
                    let dims = self.get_integer_dimensions_of(id);
                    let content_dims = self.get_content_integer_dimensions(&UIElementID::Index(id));
                    let background = match element.background.get(triple.0) {
                        Some(cool) => Some(cool.clone()),
                        None => None
                    };
                    let content_background = match element.content_background.get(triple.2) {
                        Some(cool) => Some(cool.clone()),
                        None => None
                    };
                    let content = match element.content.get(triple.1) {
                        Some(cool) => Some(cool.clone()),
                        None => None
                    };
                    (dims, content_dims, background, content, content_background)
                };

                Some(self.create_cache(dims, content_dims, background, content, content_background))
            }
            else {
                None
            };
            match new_element {
                Some(cache_element) => {
                    
                    let element = &self.elements[id];
                    let mut cache = element.cache.write().unwrap();
                    cache.insert(triple, cache_element);},
                None => ()
            }
        }
    }
    pub fn create_cache(&mut self, outside_dims:IntegerUIVector, content_dims:IntegerUIVector, background:Option<UIElementBackground>, content:Option<UIElementContent>, content_background:Option<UIElementBackground>) -> UIElementCache {
        let start_of_content = IntegerUIVector { x:(outside_dims.x - content_dims.x) / 2, y:(outside_dims.y - content_dims.y) / 2};
        let mut data = vec![0 ; (outside_dims.x * outside_dims.y) as usize];
        match background {
            Some(background) => 
            match background {
                UIElementBackground::Color(col) => {
                    data.copy_from_slice(&vec![col ; (outside_dims.x * outside_dims.y) as usize]);
                },
                UIElementBackground::Image(image_name) => {
                    match self.images.images.get(&image_name) {
                        Some(image) => {
                            data.copy_from_slice(&image.render_to_canvas(outside_dims.x as usize, outside_dims.y as usize));
                        },
                        None => ()
                    }
                }
            },
            None => ()
        }
        match content_background {
            Some(content_background) => 
            match content_background {
                UIElementBackground::Color(col) => {
    
                    let slice_range = (start_of_content.x as usize, (start_of_content.x + content_dims.x) as usize);
                    let slice_copy = vec![col ; slice_range.1 - slice_range.0];
                    for y in start_of_content.y..start_of_content.y + content_dims.y {
                        data[(slice_range.0 + (y * outside_dims.x) as usize)..(slice_range.1 + (y * outside_dims.x) as usize)].copy_from_slice(&slice_copy);
                    }
                },
                UIElementBackground::Image(image_name) => {
                    match self.images.images.get(&image_name) {
                        Some(image) => {
                            let image_data = image.render_to_canvas(outside_dims.x as usize, outside_dims.y as usize);
    
                            let slice_range = (start_of_content.x as usize, (start_of_content.x + content_dims.x) as usize);
                            for y in start_of_content.y..start_of_content.y + content_dims.y {
                                let instant_slice = (slice_range.0 + (y * outside_dims.x) as usize)..(slice_range.1 + (y * outside_dims.x) as usize);
                                data[instant_slice.clone()].copy_from_slice(&image_data[instant_slice]);
                            }
                        },
                        None => ()
                    }
                }
            },
            None => ()
        }
        match content {
            Some(content) => 
            match content {
                UIElementContent::Image(image_name) => {
                    match self.images.images.get(&image_name) {
                        Some(image) => {
                            let image_data = image.render_to_canvas(outside_dims.x as usize, outside_dims.y as usize);
    
                            let slice_range = (start_of_content.x as usize, (start_of_content.x + content_dims.x) as usize);
                            for y in start_of_content.y..start_of_content.y + content_dims.y {
                                let instant_slice = (slice_range.0 + (y * outside_dims.x) as usize)..(slice_range.1 + (y * outside_dims.x) as usize);
                                data[instant_slice.clone()].copy_from_slice(&image_data[instant_slice]);
                            }
                        },
                        None => ()
                    }
                },
                UIElementContent::Text {text, font, metrics, color, centering} => {
                    
                    //dbg!(text.clone());
                    let mut buffer = Buffer::new(&mut self.font_system, metrics);
                    Align::Center;
                    let mut buffer = buffer.borrow_with(&mut self.font_system);
                    
                    
                    let attrs = Attrs::new().family(Family::Cursive);
                    //println!("{}", text.clone());
                    buffer.set_text(&text.as_str(), attrs, cosmic_text::Shaping::Advanced);
                    let mut add_vertical = 0;
                    let mut print_pixels = false;
                    match centering {
                        TextCentering::Both => {
                            for line in &mut buffer.lines {
                                //dbg!(line.align());
                                //line.set_align(None);
                                line.set_align(Some(Align::Center));
                            }
                            add_vertical = content_dims.y/2 - (((text.trim().lines().into_iter().size_hint().0 as f32) * 0.5) * metrics.line_height) as i32 - metrics.line_height as i32;
                            print_pixels = true;
                        },
                        _ => ()
                    }
                    buffer.set_size(Some(content_dims.x as f32), Some(content_dims.y as f32));
                    dbg!(content_dims.x, content_dims.y);
                    buffer.shape_until_scroll(true);
                    
                    buffer.draw(&mut self.cache, color, |x,y,width,height, color| {
                        //data[(x + start_of_content.x as i32) as usize + ((y + start_of_content.y as i32) * outside_dims.x) as usize] = rgb_to_argb((color.r(), color.g(), color.b()));
                        let buffer_pos = (x + start_of_content.x as i32) as usize + ((y + start_of_content.y + add_vertical as i32) * outside_dims.x) as usize;
                        //if print_pixels {
                            //dbg!(x, y);
                        //}
                        if buffer_pos < data.len() {
                            let rgba_color = {
                                let r0 = color.r() as f32/255.0;
                                let g0 = color.g() as f32/255.0;
                                let b0 = color.b() as f32/255.0;
                                let a0 = color.a() as f32/255.0;
                                let (r1, g1, b1) = rgbu_to_rgbf(argb_to_rgb(data[buffer_pos]));
                                let a1 = 1.0;
                                let a01 = (1.0 - a0) * a1 + a0;
                                let r01 = ((1.0 - a0) * a1 * r1 + a0 * r0)/a01;
                                let g01 = ((1.0 - a0) * a1 * g1 + a0 * g0)/a01;
                                let b01 = ((1.0 - a0) * a1 * b1 + a0 * b0)/a01;
                                rgb_to_argb(((r01 * 255.0) as u8, (g01 * 255.0) as u8, (b01 * 255.0) as u8))
                            };
                            for yc in y..y + height as i32 {
                                for xc in x..x + width as i32 {
                                    let index = (xc + start_of_content.x as i32) as usize + ((yc + start_of_content.y as i32  + add_vertical) * outside_dims.x) as usize;
                                    if index < data.len() {
                                        data[index] = rgba_color;
                                    }
                                    
                                }
                            }
                        }
                        
                        
                        //println!("{} {} {} {}", x, y, width, height);
                        
                    });
                    //if print_pixels {
                    //    panic!("LOOKATIT");
                    //}
                }
            },
            None => ()
        }
        
        UIElementCache { data, width: outside_dims.x as usize, length: outside_dims.y as usize }
    }
    pub fn do_everything_read(&mut self) {
        self.mouse_pos.update_local();
        let (x, y) = (self.mouse_pos.get_current_state().x, self.mouse_pos.get_current_state().y);
        //println!("{} {}", x, y);
        self.apply_full_updates();
        let triples = self.get_all_display_triples(x, y);
        self.build_cache(triples);

    }
    
}

pub struct SimpleUIWrite<'a, UE:UserEvent> {
    elements:RwLockWriteGuard<'a, Vec<UIElement<UE>>>,
    images:RwLockWriteGuard<'a, UIImages>,
    framebuf:RwLockWriteGuard<'a, SyncUnsafeHordeFramebuffer>,
    user_events:Sender<UE>,
    full_updates:Receiver<(usize, usize)>
}

impl<'a, UE:UserEvent> SimpleUIWrite<'a, UE> {
    pub fn get_integer_origin_of(&self, start_elt:usize) -> IntegerUIVector {
        let start = &self.elements[start_elt];
        IntegerUIVector {
            x: match start.origin.x {
                UIUnit::RelativeToParentOrigin(value) => value + self.get_integer_origin_of_parent(start.parent.clone()).x,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).x,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).x,
            },
            y: match start.origin.y {
                UIUnit::RelativeToParentOrigin(value) => value + self.get_integer_origin_of_parent(start.parent.clone()).y,
                UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).y,
                UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32 + self.get_integer_origin_of_parent(start.parent.clone()).y,
            },
        }
    }
    pub fn does_element_exist(&self, id:&UIElementID) -> bool {
        match id {
            UIElementID::Index(idx) => *idx < self.elements.len(),
            UIElementID::Name(name) => self.elements.iter().find(|elt| {&elt.name == name}).is_some()
        }
    }
    pub fn get_integer_origin_of_parent(&self, parent:Option<UIElementID>) -> IntegerUIVector {
        match &parent {
            Some(id) => self.get_integer_origin_of(self.get_element_by_id(id)),
            None => IntegerUIVector { x: 0, y:0 }
        }
    }
    pub fn get_integer_dimensions_of(&self, start_elt:usize) -> IntegerUIVector {
        let start = &self.elements[start_elt];
        match &start.dimensions {
            UIDimensions::Decided(vector) => IntegerUIVector {
                x: match vector.x {
                    UIUnit::RelativeToParentOrigin(value) => value,
                    UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32,
                    UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32,
                },
                y: match vector.y {
                    UIUnit::RelativeToParentOrigin(value) => value,
                    UIUnit::ParentHeightProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).y as f32 * prop) as i32,
                    UIUnit::ParentWidthProportion(prop) => (self.get_integer_dimensions_of_parent(start.parent.clone()).x as f32 * prop) as i32,
                },
            },
        }
    }
    pub fn get_integer_dimensions_of_parent(&self, parent:Option<UIElementID>) -> IntegerUIVector {
        match &parent {
            Some(id) => self.get_integer_dimensions_of(self.get_element_by_id(id)),
            None => IntegerUIVector { x: self.framebuf.get_dims().get_width() as i32, y: self.framebuf.get_dims().get_height() as i32 }
        }
    }
    pub fn get_element_by_id(&self, id:&UIElementID) -> usize {
        match id {
            UIElementID::Index(given) => *given,
            UIElementID::Name(name) => {
                for (i, element) in self.elements.iter().enumerate() {
                    if &element.name == name {
                        return i
                    }
                }
                panic!("No element matching name {} found !", name.clone());
            }
        }
    }
    pub fn draw_to_screen(&mut self, id:&UIElementID, triplet:(usize, usize, usize)) {
        let id = self.get_element_by_id(id);
        let elt = &self.elements[id];
        let screen_dims = self.framebuf.get_dims();
        match elt.cache.read().unwrap().get(&triplet) {
            Some(cache_element) => {
                let orig = self.get_integer_origin_of(id);
                let dims = self.get_integer_dimensions_of(id);
                //dbg!(dims.clone());
                let slice_range = (orig.x as usize, (orig.x + dims.x) as usize);
                let mut data = self.framebuf.get_other_data();
                for y in orig.y..orig.y + dims.y {
                    let instant_slice = (slice_range.0 + (y * screen_dims.get_width() as i32) as usize)..(slice_range.1 + (y * screen_dims.get_width() as i32) as usize);
                    //dbg!(instant_slice.clone());
                    if instant_slice.start < data.len() && instant_slice.end < data.len() {
                        data[instant_slice.clone()].copy_from_slice(&cache_element.data[(((y - orig.y) * dims.x) as usize)..(dims.x as usize + ((y - orig.y) * dims.x) as usize)]);
                    }
                    
                }
            },
            None => ()
        }
    }
    pub fn draw_all(&mut self) {
        for i in 0..self.elements.len() {
            let elt = &self.elements[i];
            if elt.visible.load(Ordering::Relaxed) {
                self.draw_to_screen(&UIElementID::Index(i), (elt.chosen_triplet.0.load(Ordering::Relaxed) as usize, elt.chosen_triplet.1.load(Ordering::Relaxed) as usize, elt.chosen_triplet.2.load(Ordering::Relaxed) as usize));
            }
        }
    }
}

pub struct UIBoundingBox {
    x1:i32,
    y1:i32,
    x2:i32,
    y2:i32
}

impl UIBoundingBox {
    pub fn new(x1:i32, y1:i32, x2:i32, y2:i32) -> Self {
        Self { x1:if x1 < x2 {x1} else {x2}, y1:if y1 < y2 {y1} else {y2}, x2:if x2 < x1 {x1} else {x2}, y2:if y2 < y1 {y1} else {y2} }
    }
    pub fn is_in(&self, x:i32, y:i32) -> bool {
        self.x1 <= x && self.x2 >= x && self.y1 <= y && self.y2 >= y
    }
}

pub struct SimpleUISave<UE:UserEvent> {
    elements:Vec<UIElement<UE>>,
    images:UIImages
}

impl<UE:UserEvent> SimpleUI<UE> {
    pub fn get_current_elements(&self) -> Vec<UIElement<UE>> {
        self.elements.read().unwrap().clone()
    }
    pub fn get_save_of_current(&self) -> SimpleUISave<UE> {
        SimpleUISave { elements: self.elements.read().unwrap().clone(), images:self.images.read().unwrap().clone() }
    }
    pub fn swap_save(&self, save:SimpleUISave<UE>) -> SimpleUISave<UE> {
        let new_save = self.get_save_of_current();
        self.load_save(save);
        new_save
    }
    pub fn load_save(&self, save:SimpleUISave<UE>) {
        *self.elements.write().unwrap() = save.elements;
        *self.images.write().unwrap() = save.images;
    }
    pub fn get_read<'a>(&'a self) -> SimpleUIRead<'a, UE> {
        SimpleUIRead {full_updates:self.full_updates.clone(), elements: self.elements.read().unwrap(), images: self.images.read().unwrap(), user_events: self.user_events.clone(), framebuf:self.framebuf.read().unwrap(), cache:self.cache.write().unwrap(), font_system:self.font_system.write().unwrap(), mouse_pos:self.mouse_pos.clone() }
    }
    pub fn get_write<'a>(&'a self) -> SimpleUIWrite<'a, UE> {
        SimpleUIWrite {full_updates:self.full_updates.clone(), elements: self.elements.write().unwrap(), images: self.images.write().unwrap(), user_events: self.user_events.clone(), framebuf:self.framebuf.write().unwrap() }
    }
    pub fn add_element(&mut self, element:UIElement<UE>) {
        self.elements.write().unwrap().push(element);
    }
    pub fn change_position_of(&mut self, element:UIElementID, new_pos:UIVector) -> Option<usize> {
        let mut write = self.get_write();
        let id = element;
        if write.does_element_exist(&id) {
            let id = write.get_element_by_id(&id);
            write.elements[id].origin = new_pos;
            Some(id)
        }
        else {
            None
        }
    }
    pub fn change_dimensions_of(&mut self, element:UIElementID, new_dims:UIDimensions) -> Option<usize> {
        let mut write = self.get_write();
        if write.does_element_exist(&element) {
            let id = write.get_element_by_id(&element);
            //dbg!(write.elements[id].dimensions.clone());
            //dbg!(new_dims.clone());
            //dbg!(write.elements[id].cache.read().unwrap().len());
            write.elements[id].dimensions = new_dims;
            write.elements[id].cache.write().unwrap().clear();
            for child in &write.elements[id].children {
                let child_id = write.get_element_by_id(child);
                //dbg!(write.elements[child_id].cache.read().unwrap().len());
                write.elements[child_id].cache.write().unwrap().clear();
                
                // Not recursive, must be fixed... Eventually
            }
            //panic!("");
            Some(id)
        }
        else {
            None
        }
    }
    pub fn add_many_connected_elements(&mut self, new_elements:Vec<UIElement<UE>>) {
        let len = self.elements.read().unwrap().len();
        let mut elements = self.elements.write().unwrap();
        for mut element in new_elements {
            element.parent = match element.parent {
                Some(id) => Some(self.correct_id_to_local(id, len)),
                None => None
            };
            for child in element.children.iter_mut() {
                *child = self.correct_id_to_local(child.clone(), len)
            }
            elements.push(element);
        }
    }
    pub fn add_multiple_widgets(&mut self, new_widgets:Vec<Vec<UIElement<UE>>>) {
        for widget in new_widgets {
            self.add_many_connected_elements(widget);
        }
    }
    pub fn change_visibility_of_group(&mut self, elements:Vec<UIElement<UE>>, new_vis:bool) -> Vec<usize> {
        let mut found = Vec::with_capacity(elements.len());
        let mut write = self.get_write();
        for element in &elements {
            let id = UIElementID::Name(element.name.clone());
            if write.does_element_exist(&id) {
                let id = write.get_element_by_id(&id);
                found.push(id);
                write.elements[id].visible.store(new_vis, Ordering::Relaxed);
            }
        }
        found
    }
    pub fn change_visibility_of_widgets(&mut self, widgets:Vec<Vec<UIElement<UE>>>, new_vis:bool) {
        for widget in widgets {
            self.change_visibility_of_group(widget, new_vis);
        }
    }
    pub fn does_element_exist(&self, id:UIElementID) -> bool {
        let read = self.elements.read().unwrap();
        match id {
            UIElementID::Index(idx) => read.len() > idx,
            UIElementID::Name(name) => read.iter().find(|elt| {elt.name == name}).is_some()
        }
    }
    fn correct_id_to_local(&self, id:UIElementID, previous_len:usize) -> UIElementID {
        match id.clone() {
            UIElementID::Name(name) => id,
            UIElementID::Index(idx) => UIElementID::Index(idx + previous_len)
        }
    }
    pub fn add_image(&self, path:PathBuf, name:Option<String>) {
        self.images.write().unwrap().add_image(path, name);
    }

    pub fn add_images(&self, images:Vec<(PathBuf, Option<String>)>) {
        for (path, name) in images {
            self.add_image(path, name);
        }
    }
    pub fn add_image_from_id(&self,textures:&Textures, texture_set:usize) {
        let data = textures.get_data_with_id(&TextureSetID::ID(texture_set));
        let mipmap = data.get_mip_map(0);
        self.images.write().unwrap().add_image_from_texture(mipmap, data.nom.clone());
    }
}

#[derive(Clone)]
pub struct UIImages {
    images:HashMap<String, UIImage>,
}

fn load_image(path:PathBuf) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, ()> {
    // tries to load an image in the "textures" folder
    match Reader::open(path) {
        Ok(texture) => {
            //dbg!(nom);
            let image_buffer = texture.decode().unwrap().to_rgb8();
            return Ok(image_buffer);
        }
        Err(err) => {
            println!(
                "texture introuvable dans le dossier cartes... erreur {}", err);
            return Err(());
        }
    }
}

impl UIImages {
    pub fn add_image(&mut self, path:PathBuf, name:Option<String>) {
        match load_image(path.clone()) {
            Ok(buffer) => {
                let mut out = Vec::new();
                for y in 0..buffer.height() {
                    for x in 0..buffer.width() {
                        let col = buffer.get_pixel(x, y);
                        out.push(rgb_to_argb((col.0[0], col.0[1], col.0[2])));
                    }
                }
                match name {
                    Some(string) => self.images.insert(string, UIImage { width: buffer.width() as usize, height: buffer.height() as usize, data: out }),
                    None => self.images.insert(path.file_name().unwrap().to_str().unwrap().to_string(), UIImage { width: buffer.width() as usize, height: buffer.height() as usize, data: out }),
                };
            },
            Err(_) => panic!("Couldn't add image {}", path.to_string_lossy())
        }
    }
    pub fn add_image_from_texture(&mut self, mipmap:&MipMap, texture_name:String) {
        let raw_data = &mipmap.data;
        self.images.insert(texture_name, UIImage { width: mipmap.largeur_usize, height: mipmap.hauteur as usize, data: raw_data.clone() });
    }
}

#[derive(Clone)]
pub struct UIImage {
    width:usize,
    height:usize,
    data:Vec<u32>,
}

impl UIImage {
    fn render_to_canvas(&self, c_width:usize, c_height:usize) -> Vec<u32> {
        let mut colors = Vec::with_capacity(c_height * c_width);
        let mut xi = 0.0;
        let mut yi = 0.0;
        let mut dx = (self.width as f32)/(c_width as f32);
        let mut dy = (self.height as f32)/(c_height as f32);
        for y in 0..c_height {
            xi = 0.0;
            for x in 0..c_width {
                colors.push(self.data[yi as usize * self.width + xi as usize]);
                xi += dx;
            }
            yi += dy;
        }
        colors
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UIUserAction {
    Hovering,
    Clicking,
    Nothing,
}

pub trait UserEvent: Hash + Clone + PartialEq {

}
#[derive(Clone)]
pub enum UIEvent<UE:UserEvent> {
    ChangeBackground(usize),
    ChangeContent(usize),
    ChangeContentBackground(usize),
    User(UE),
}

#[derive(Clone)]
pub struct UIElement<UE:UserEvent> {
    origin:UIVector,
    dimensions:UIDimensions,
    padding_to_content:UIVector,
    parent:Option<UIElementID>,
    content:Vec<UIElementContent>,
    background:Vec<UIElementBackground>,
    content_background:Vec<UIElementBackground>,
    children:Vec<UIElementID>,
    reactions:Vec<(UIUserAction, UIEvent<UE>)>,
    name:String,
    cache:Arc<RwLock<HashMap<(usize, usize, usize), UIElementCache>>>,
    chosen_triplet:Arc<(AtomicIsize, AtomicIsize, AtomicIsize)>,
    visible:Arc<AtomicBool>
}

impl<UE:UserEvent> UIElement<UE> {
    pub fn new(origin:UIVector, dimensions:UIDimensions, padding_to_content:UIVector, parent:Option<UIElementID>, name:String) -> Self {
        Self { visible:Arc::new(AtomicBool::new(true)), origin, dimensions, padding_to_content, parent, content:Vec::new(), background: Vec::new(), content_background: Vec::new(), children: Vec::new(), reactions: Vec::new(), name, cache:Arc::new(RwLock::new(HashMap::new())), chosen_triplet:Arc::new((AtomicIsize::new(0), AtomicIsize::new(0), AtomicIsize::new(0))) }
    }
    pub fn with_content(mut self, content:UIElementContent) -> Self {
        self.content.push(content);
        self
    }
    pub fn with_background(mut self, background:UIElementBackground) -> Self {
        self.background.push(background);
        self
    }
    pub fn with_content_background(mut self, background:UIElementBackground) -> Self {
        self.content_background.push(background);
        self
    }
    pub fn with_reaction(mut self, reac:(UIUserAction, UIEvent<UE>)) -> Self {
        self.reactions.push(reac);
        self
    }
    pub fn with_child(mut self, child:UIElementID) -> Self {
        self.children.push(child);
        self
    }
    pub fn change_visibility(mut self, visible:bool) -> Self {
        self.visible.store(visible, Ordering::Relaxed);
        self
    }
    pub fn get_name_as_id(&self) -> UIElementID {
        UIElementID::Name(self.name.clone())
    }
}

pub struct UIElementCache {
    data:Vec<u32>,
    width:usize,
    length:usize,
}

#[derive(Clone)]
pub enum UIElementBackground {
    Color(u32),
    Image(String)
}
#[derive(Clone)]
pub enum UIElementID {
    Index(usize),
    Name(String)
}
#[derive(Clone, Copy, Debug)]
pub enum UIUnit {
    RelativeToParentOrigin(i32),
    ParentWidthProportion(f32),
    ParentHeightProportion(f32),
}
#[derive(Clone, Copy, Debug)]
pub struct UIVector {
    pub x:UIUnit,
    pub y:UIUnit,
}

impl UIVector {
    pub fn new(x:UIUnit, y:UIUnit) -> Self {
        Self { x, y }
    }
    pub fn zero() -> Self {
        Self { x: UIUnit::RelativeToParentOrigin(0), y: UIUnit::RelativeToParentOrigin(0) }
    }
}
#[derive(Clone, Debug)]
pub enum UIDimensions {
    Decided(UIVector),
}
#[derive(Clone)]
pub enum UIElementContent {
    Text {text:String, font:String, metrics:Metrics, color:Color, centering:TextCentering},
    Image(String),
}

#[derive(Clone)]
pub enum TextCentering {
    Horizontal,
    Vertical,
    Both,
    Neither
}