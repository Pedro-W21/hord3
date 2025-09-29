extern crate proc_macro;

use proc_macro::{Span, TokenStream};
use quote::{quote, __private::Span as OtherSpan};
use syn::{self, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields::{self, *}, FieldsNamed, FieldsUnnamed, Ident, Lit, LitInt, Meta, Path, Type, __private::TokenStream2};

#[proc_macro_derive(GameEngine, attributes(not_rendered, rendering_engine, rendering_engine_generic, not_multiplayer, do_multiplayer, extra_data))] 
pub fn derive_engine(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast:DeriveInput = syn::parse(input).unwrap();

    match &ast.data {
        Data::Struct(datastruct) => match &datastruct.fields {
            Fields::Named(fields) => derive_engine_on_struct(&ast, datastruct, fields),
            _ => panic!("Can only derive Engine on structs with named fields")
        }
        _ => panic!("Can only derive Engine on structs")
    }

}



fn derive_engine_on_struct(ast:&DeriveInput, data:&DataStruct, fields:&FieldsNamed) -> TokenStream {
    let engine_ident = ast.ident.clone();

    let mut ent_types = Vec::new();
    let mut ent_idents = Vec::new();

    let mut world_id = None;
    let mut world_type = None;

    let mut renderable_fields = Vec::new();
    let mut renderable_types = Vec::new();

    let mut multiplayer_ents = Vec::new();
    let mut multiplayer_ents_types = Vec::new();

    let mut extra_data = None;
    let mut extra_data_type = None;

    let mut rendering_engine = None;
    let mut do_multiplayer = false;

    for attr in &ast.attrs {
        if attr.path.is_ident(&Ident::new("do_multiplayer", OtherSpan::call_site())) {
            do_multiplayer = true;
        }
        else {
            match attr.parse_meta() {
                Ok(meta) => match meta {
                    Meta::NameValue(name_value) => {
                        match name_value.path.get_ident() {
                            Some(ident) => {
                                let id = ident.to_string();
                                let str_ident = id.trim();
                                match str_ident {
                                    "rendering_engine" => match name_value.lit {
                                        Lit::Str(value) => {
                                            rendering_engine = Some((
                                                Ident::new(value.value().to_lowercase().trim(), OtherSpan::call_site()),
                                                Ident::new(value.value().trim(), OtherSpan::call_site()),
                                                None
                                            )
                                            );
                                        },
                                        _ => panic!("rendering engine type has to be a string"),
                                    },
                                    "rendering_engine_generic" => match name_value.lit {
                                        Lit::Str(value) => {
                                            match &mut rendering_engine {
                                                Some((_,_,a)) => {
                                                    *a = Some(Ident::new(value.value().trim(), OtherSpan::call_site()));
                                                },
                                                None => panic!("NOT SUPPOSED TO COME BEFORE OR WITHOUT RENDERING_ENGINE")
                                            };
                                        },
                                        _ => panic!("rendering engine type has to be a string"),
                                    },
    
                                    _ => ()
                                }
                            },
                            None => panic!("Meta name = value for attribute doesn't have simple name path")
                        }
                    },
                    _ => panic!("Meta for attribute isn't name = value pair")
                },
                Err(error) => panic!("Error while parsing attributes : {}", error)
            }
        }
        
        
    }


    for field in &fields.named {
        
        if field.ident.as_ref().unwrap().to_string() == String::from("world") {
            world_id = Some(field.ident.as_ref().unwrap().clone());
            world_type = Some(field.ty.clone());
        }
        else {
            let mut not_renderable = false;
            let mut not_multiplayer = false;
            let mut is_extra_data = false;

            for attr in &field.attrs {
                if attr.path.is_ident(&Ident::new("not_rendered", OtherSpan::call_site())) {
                    not_renderable = true;
                }
                if attr.path.is_ident(&Ident::new("not_multiplayer", OtherSpan::call_site())) {
                    not_multiplayer = true;
                }
                if attr.path.is_ident(&Ident::new("extra_data", OtherSpan::call_site())) {
                    is_extra_data = true;
                }
            }
            if is_extra_data {
                match extra_data {
                    Some(_) => panic!("Multiple Extra data ? more likely than you'd think !"),
                    None => {
                        extra_data = Some(field.ident.as_ref().unwrap().clone());
                        extra_data_type = Some(field.ty.clone());
                    }
                }
            }
            else {

                ent_idents.push(field.ident.as_ref().unwrap().clone());
                ent_types.push(field.ty.clone());
                if !not_multiplayer && do_multiplayer {
                    multiplayer_ents.push(field.ident.as_ref().unwrap().clone());
                    multiplayer_ents_types.push(field.ty.clone());
                }
                if !not_renderable {
                    renderable_fields.push(field.ident.as_ref().unwrap().clone());
                    renderable_types.push(field.ty.clone());
                }
            }
            
        }
    }

    let user_data = match world_id {
        Some(id) => UserGivenData {
            world_id:id,
            world_type:world_type.unwrap(),
            ent_idents,
            ent_types,
            engine_ident,
            renderable_fields,
            renderable_types,
            rendering_engine,
            multiplayer_ents,
            multiplayer_ents_types,
            extra_data,
            extra_data_type
        },
        None => panic!("No world type for engine, aborting codegen")
    };

    create_engine(ast, data, fields, &user_data).into()

}

struct UserGivenData {
    world_id:Ident,
    world_type:Type,
    ent_idents:Vec<Ident>,
    ent_types:Vec<Type>,
    engine_ident:Ident,
    renderable_fields:Vec<Ident>,
    renderable_types:Vec<Type>,
    rendering_engine:Option<(Ident, Ident, Option<Ident>)>,
    multiplayer_ents:Vec<Ident>,
    multiplayer_ents_types:Vec<Type>,
    extra_data:Option<Ident>,
    extra_data_type:Option<Type>
}

fn create_engine(ast:&DeriveInput, data:&DataStruct, fields:&FieldsNamed, user_data:&UserGivenData) -> TokenStream2 {

    let mut numbers = Vec::with_capacity(user_data.ent_idents.len());
    for i in 0..user_data.ent_idents.len() {
        numbers.push(Lit::Int(LitInt::new(&format!("{}", i).trim(), OtherSpan::call_site())));
    }
    let mut max_number = Lit::Int(LitInt::new(&format!("{}", user_data.ent_idents.len()).trim(), OtherSpan::call_site()));
    let engine_struct_ident = Ident::new(format!("{}Base", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let engine_compute_ident = Ident::new(format!("{}Compute", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let engine_reader_ident = Ident::new(format!("{}Reader", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let special_trait_ident = Ident::new(format!("{}Controller", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let mut generic_ent_idents = Vec::with_capacity(user_data.ent_idents.len());
    let mut entity_handler_names = Vec::with_capacity(user_data.ent_idents.len());
    let world_handler_name = Ident::new(format!("{}_whandler", user_data.world_id.to_string().trim()).trim(), OtherSpan::call_site());
    let mut entity_reader_names = Vec::with_capacity(user_data.ent_idents.len());
    for i in 0..user_data.ent_idents.len() {
        generic_ent_idents.push(Ident::new(format!("E{i}").trim(), OtherSpan::call_site()));
        entity_handler_names.push(Ident::new(format!("{}_handler", user_data.ent_idents[i].to_string().trim()).trim(), OtherSpan::call_site()));
        match user_data.ent_types[i].clone() {
            Type::Path(cool_path) => match cool_path.path.get_ident() {
                Some(nice_ident) => {
                    entity_reader_names.push(Ident::new(format!("{}VecRead", nice_ident.to_string().trim()).trim(), OtherSpan::call_site()))
                },
                None => panic!("Couldn't get ident from type")
            },
            _ => panic!("Can't have that type in an engine derive")
        }
        
    }
    let ent_idents = &user_data.ent_idents;
    let world_type = &user_data.world_type;
    let ent_types = &user_data.ent_types;
    let world_id = &user_data.world_id;
    let first_ent = &user_data.ent_idents[0];
    let first_ent_type = &user_data.ent_types[0];
    let engine_ident = user_data.engine_ident.clone();
    let (rendering_struct_addon, rendering_type_ident, rendering_func, rendering_task) = match &user_data.rendering_engine {
        Some((rident, rtype, gen_type)) => {
            let renderable_fields = &user_data.renderable_fields;
            match gen_type {
                Some(gen_ident) => (
                    quote! {#rident:std::sync::Arc<#rtype <#gen_ident>>,},
                    quote! {#rident,},
                    quote! {
                        fn update_all_renderings(&mut self) {
                            let mut writer = self.#rident.get_write();
                            #(self.#renderable_fields.do_all_renders(&mut writer));*;
                            WorldWriteHandler::from_world_handler(&self.#world_id).world.do_render_changes(&mut writer);
                        }
                    },
                    quote! {
                        3 => self.update_all_renderings(),
                    }
                ),
                None => (
                    quote! {#rident:std::sync::Arc<#rtype>,},
                    quote! {#rident,},
                    quote! {
                        fn update_all_renderings(&mut self) {
                            let mut writer = self.#rident.get_write();
                            #(self.#renderable_fields.do_all_renders(&mut writer));*;
                            WorldWriteHandler::from_world_handler(&self.#world_id).world.do_render_changes(&mut writer);
                        }
                    },
                    quote! {
                        3 => self.update_all_renderings(),
                    }
                )
            }
            
        },
        None => {
            (
                quote! {},
                quote! {},
                quote! {},
                quote! {}
            )
        }
    };
    
    //panic!("PANICKED {}", extra_func_get_addon.to_string());
    let mut engine_reader_writer_ident = Ident::new(format!("{}ReadWrite", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let mut engine_reader_ident = Ident::new(format!("{}Reader", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let mut engine_writer_ident = Ident::new(format!("{}Writer", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());

    let mut entity_reader_idents = Vec::with_capacity(user_data.ent_idents.len());
    let mut entity_writer_idents = Vec::with_capacity(user_data.ent_idents.len());
    for ent_type in &user_data.ent_types {
        match ent_type {
            Type::Path(cool_path) => match cool_path.path.get_ident() {
                Some(nice_ident) => {
                    entity_reader_idents.push(Ident::new(format!("{}VecRead",  nice_ident.to_string().trim()).trim(), OtherSpan::call_site()));
                    entity_writer_idents.push(Ident::new(format!("{}VecWrite",  nice_ident.to_string().trim()).trim(), OtherSpan::call_site()));
                },
                None => panic!("Couldn't get ident from type")
            },
            _ => panic!("Can't have that type in an engine derive")
        }
        
    }
    let mut total_id_ident = Ident::new(format!("{}TID", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
    let (multiplayer_struct_addon, multiplayer_type_ident, multiplayer_func, multiplayer_task, multiplayer_creator, total_id_definition, total_id_struct_ident, world_multiplayer_handling, multiplayer_new_addon, entity_multiplayer_handling) = if user_data.multiplayer_ents.len() > 0 {
        
        let mut total_id_ident = Ident::new(format!("{}TID", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        let mut total_component_ident = Ident::new(format!("{}GC", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        let mut total_event_ident = Ident::new(format!("{}GE", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        let mut total_event_variant_ident = Ident::new(format!("{}GEVariant", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        
        (   
            // multiplayer_struct_addon
            quote! {
                pub multiplayer:HordeMultiplayer<#engine_reader_writer_ident>,
                tick:std::sync::Arc<AtomicUsize>,
                current_tick_over:std::sync::Arc<AtomicUsize>,
                world_events:std::sync::Arc<std::sync::RwLock<Vec<<#world_type as World<#total_id_ident>>::WE>>>,
                all_world_events:std::sync::Arc<std::sync::RwLock<Vec<<#world_type as World<#total_id_ident>>::WE>>>,
                general_events_sender:std::sync::mpmc::Sender<#total_event_ident>,
                is_server:bool,
            },
            // multiplayer_type_ident
            quote! {
                multiplayer,//HordeMultiplayer<#engine_reader_writer_ident>
                tick:std::sync::Arc::new(AtomicUsize::new(0)),
                current_tick_over:std::sync::Arc::new(AtomicUsize::new(0)),
                world_events:std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(128))),
                all_world_events:std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(128))),
                general_events_sender:sender,
                is_server,
            },
            // multiplayer_func
            quote! {
                /// Sequential, only call once per tick at the end of the game logic
                pub fn send_must_send(&mut self) {
                    let tick = self.tick.load(Ordering::Relaxed);
                    #({
                        let mut writer = self.#ent_idents.get_need_sync();
                        for event in writer.iter() {
                            self.general_events_sender.send(#total_event_ident {variant:#total_event_variant_ident::#ent_idents(event.clone()) ,tick});
                        }
                        writer.clear();
                    });*;
                    {
                        let mut world_writer = self.world_events.write().unwrap();
                        for event in world_writer.iter() {
                            self.general_events_sender.send(#total_event_ident {variant:#total_event_variant_ident::#world_id(event.clone()) ,tick});
                        }
                        world_writer.clear();
                    }
                    self.tick.fetch_add(1, Ordering::Relaxed);
                }
            },
            // multiplayer_task
            quote! {
                10 => {
                    let mut rwter = #engine_reader_writer_ident::get_from_engine(&self);
                    self.multiplayer.handshakes_players_events(&mut rwter);
                },
                11 => {
                    let mut rwter = #engine_reader_writer_ident::get_from_engine(&self);
                    self.multiplayer.streams_share_spread(&rwter);
                },
                12 => self.multiplayer.reset_server_counters(),
                13 => self.multiplayer.send_held_up_packets(),
                20 => {
                    let mut rwter = #engine_reader_writer_ident::get_from_engine(&self);
                    self.multiplayer.receive_all_events_and_respond(&mut rwter);
                },
                21 => self.multiplayer.send_all_events(),
                30 => {
                    self.send_must_send();
                }
            },
            // multiplayer_creator
            quote! {
                let (sender, receiver) = std::sync::mpmc::channel();
                let is_server = match multi_choice.clone() {
                    HordeMultiModeChoice::Client {..} => false,
                    _ => true
                };
                let multiplayer = HordeMultiplayer::new(multi_choice, receiver);
            },
            // total_id_definition
            quote! {
                #[derive(Clone)]
                pub struct #engine_reader_writer_ident {
                    #(#ent_idents:<#ent_types as Entity<#total_id_ident>>::EV<#total_id_ident>),*,
                    #world_id:WorldHandler<#world_type, #total_id_ident>,
                    tick:std::sync::Arc<AtomicUsize>,
                    current_tick_over:std::sync::Arc<AtomicUsize>,
                    world_events:std::sync::Arc<std::sync::RwLock<Vec<<#world_type as World<#total_id_ident>>::WE>>>,
                    all_world_events:std::sync::Arc<std::sync::RwLock<Vec<<#world_type as World<#total_id_ident>>::WE>>>,
                }
                pub struct #engine_reader_ident <'a> {
                    #(#ent_idents:#entity_reader_idents<'a, #total_id_ident>),*,
                    #world_id:WorldComputeHandler<'a, #world_type, #total_id_ident>,
                }
                pub struct #engine_writer_ident <'a> {
                    #(#ent_idents:#entity_writer_idents<'a, #total_id_ident>),*,
                    #world_id:WorldWriteHandler<'a, #world_type, #total_id_ident>,
                }


                impl #engine_reader_writer_ident {
                    fn get_write<'a>(&'a mut self) -> #engine_writer_ident <'a> {
                        #engine_writer_ident {
                            #(#ent_idents:self.#ent_idents.get_write()),*,
                            #world_id:WorldWriteHandler::from_world_handler(&self.#world_id),
                        }
                    }
                    fn get_read<'a>(&'a self) -> #engine_reader_ident<'a> {
                        #engine_reader_ident {
                            #(#ent_idents:self.#ent_idents.get_read()),*,
                            #world_id:WorldComputeHandler::from_world_handler(&self.#world_id),    
                        }
                    }
                    fn get_from_engine(engine:&#engine_struct_ident) -> Self {
                        Self {
                            #(#ent_idents:engine.#ent_idents.clone()),*,
                            #world_id:engine.#world_id.clone(),
                            tick:engine.tick.clone(),
                            current_tick_over:engine.current_tick_over.clone(),
                            world_events:engine.world_events.clone(),
                            all_world_events:engine.all_world_events.clone()
                        }
                    }
                }
                #[derive(Clone, PartialEq, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes)]
                pub enum #total_component_ident {
                    #(#ent_idents(<#ent_types as MultiplayerEntity<#total_id_ident>>::GEC)),*,
                    #world_id(#world_type)
                }

                impl GlobalComponent for #total_component_ident {

                }

                #[derive(Clone, PartialEq, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes)]
                pub enum #total_event_variant_ident {
                    #(#ent_idents(<#ent_types as MultiplayerEntity<#total_id_ident>>::GEV<#total_id_ident>)),*,
                    #world_id(<#world_type as World<#total_id_ident>>::WE)
                }
                #[derive(Clone, PartialEq, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes)]
                pub struct #total_event_ident {
                    variant:#total_event_variant_ident,
                    tick:usize,
                }
                impl MultiplayerEngine for #engine_reader_writer_ident {
                    type GE = #total_event_ident;
                    type ID = #total_id_ident;
                    type RIDG = fastrand::Rng;
                    fn set_component(&mut self, id:Self::ID, value:<Self::GE as GlobalEvent>::GC) {
                        match id {
                            #(#total_id_ident::#ent_idents(identity) => match value {
                                #total_component_ident::#ent_idents(component) => self.#ent_idents.change_component(component, identity.clone()),
                                _ => panic!("Wrong ID for component to set")
                            }
                            ),*,
                            #total_id_ident::#world_id => match value {
                                #total_component_ident::#world_id(new_world) => *WorldWriteHandler::from_world_handler(&self.#world_id).world = new_world,
                                _ => panic!("Wrong ID for component to set")
                            }
                        }
                    }
                    fn set_world(&mut self, world:<Self::GE as GlobalEvent>::WD) {
                        *WorldWriteHandler::from_world_handler(&self.#world_id).world = world;
                    }
                    fn is_that_component_correct(&self, id:&Self::ID, component:&<Self::GE as GlobalEvent>::GC) -> bool {
                        match id {
                            #(#total_id_ident::#ent_idents(identity) => match component {
                                #total_component_ident::#ent_idents(compo) => self.#ent_idents.is_that_component_correct(compo.clone(), identity.clone()),
                                _ => panic!("Asked to verify bad component")
                            }
                            ),*,
                            #total_id_ident::#world_id => panic!("Asked to verify world")
                        }
                    }
                    fn get_event_origin(&self, event:&Self::GE) -> Option<Self::ID> {
                        match &event.variant {
                            #(#total_event_variant_ident::#ent_idents(sub_event) => sub_event.get_source()),*,
                            #total_event_variant_ident::#world_id(world_event) => world_event.get_source(),
                        }
                    }
                    fn get_tick(&self, event:&Self::GE) -> usize {
                        event.tick 
                    }
                    fn get_target(event:&Self::GE) -> Self::ID {
                        match &event.variant {
                            #(#total_event_variant_ident::#ent_idents(sub_event) => #total_id_ident::#ent_idents(sub_event.get_id())),*,
                            #total_event_variant_ident::#world_id(world_event) => #total_id_ident::#world_id
                        }
                    }
                    fn get_components_to_sync_for(&self, id:&Self::ID) -> Vec<#total_component_ident> {
                        let read = self.get_read();
                        let mut components = Vec::with_capacity(8);
                        match id {
                            #(#total_id_ident::#ent_idents(identity) => read.#ent_idents.get_components_for(identity.clone()).into_iter().for_each(|component| {components.push(#total_component_ident::#ent_idents(component.clone()))} )),*,
                            #total_id_ident::#world_id => components.push(#total_component_ident::#world_id(read.#world_id.world.clone()))
                        }
                        components
                    }
                    fn apply_event(&mut self, event:Self::GE) {
                        
                        match &event.variant {
                            #(#total_event_variant_ident::#ent_idents(sub_event) => self.#ent_idents.apply_one_event(sub_event.clone())),*,
                            #total_event_variant_ident::#world_id(world_event) => {let mut writer = self.get_write(); <<#world_type as World<#total_id_ident>>::WE as WorldEvent<#world_type, #total_id_ident>>::apply_event(world_event.clone(), &mut writer.#world_id.world)}
                        }
                    }
                    fn get_all_components_and_world(&self) -> (Vec<(Self::ID, <Self::GE as GlobalEvent>::GC)>, <Self::GE as GlobalEvent>::WD) {
                        let read = self.get_read();
                        let mut components = Vec::with_capacity(#(read.#ent_idents.get_expected_len())+*);
                        #(
                        for i in 0..read.#ent_idents.get_expected_len() {
                            let mut components_to_sync_for = self.get_components_to_sync_for(&#total_id_ident::#ent_idents(i));
                            for compo in components_to_sync_for {
                                components.push((#total_id_ident::#ent_idents(i), compo))
                            }
                        }
                        );*;
                        (
                            components,
                            read.#world_id.world.clone()
                        )
                    }
                    fn get_random_id_generator() -> Self::RIDG {
                        fastrand::Rng::new()
                    }
                    fn get_total_len(&self) -> usize {
                        let read = self.get_read();
                        let len = #(
                            read.#ent_idents.get_expected_len()
                        )+*;
                        len                        
                    }
                    fn generate_random_id(&self, generator:&mut Self::RIDG) -> Option<Self::ID> {
                        let read = self.get_read();
                        if #max_number > 0 {
                            match generator.usize(0..#max_number) {
                                #(#numbers if read.#ent_idents.get_expected_len() > 0 => Some(#total_id_ident::#ent_idents(generator.usize(0..read.#ent_idents.get_expected_len())))),*,
                                _ => None
                            }
                        }
                        else {
                            None
                        }
                    }
                    fn get_latest_event_report(&self) -> Option<HordeEventReport<Self::ID, Self::GE>> {
                        if self.current_tick_over.load(Ordering::Relaxed) >= self.tick.load(Ordering::Relaxed) {
                            None
                        }
                        else {
                            let event_map_capacity = {
                                let read = self.get_read();
                                #(read.#ent_idents.get_expected_len())+*
                            };
                            let mut event_map:HashMap<Self::ID, Vec<Self::GE>> = HashMap::with_capacity(event_map_capacity);
                            let tick = self.current_tick_over.load(Ordering::Relaxed);
                            #({
                                let mut event_writer = self.#ent_idents.get_all_events();
                                for event in event_writer.iter() {
                                    match event.get_source() {
                                        Some(global_id) => match event_map.get_mut(&global_id) {
                                            Some(mut vector) => vector.push(#total_event_ident {variant:#total_event_variant_ident::#ent_idents(event.clone()), tick}),
                                            None => {event_map.insert(global_id.clone(), vec![#total_event_ident {variant:#total_event_variant_ident::#ent_idents(event.clone()), tick}]);}
                                        },
                                        None => ()
                                    }
                                }
                                event_writer.clear();
                            });*;
                            {
                                let mut world_event_writer = self.all_world_events.write().unwrap();
                                for event in world_event_writer.iter() {
                                    match event.get_source() {
                                        Some(global_id) => match event_map.get_mut(&global_id) {
                                            Some(mut vector) => vector.push(#total_event_ident {variant:#total_event_variant_ident::#world_id(event.clone()), tick}),
                                            None => {event_map.insert(global_id.clone(), vec![#total_event_ident {variant:#total_event_variant_ident::#world_id(event.clone()), tick}]);}
                                        },
                                        None => ()
                                    }
                                }
                                world_event_writer.clear();
                            }
                            let current_tick = self.current_tick_over.fetch_add(1, Ordering::Relaxed);
                            Some(HordeEventReport::new(event_map, current_tick))
                        }
                    }
                }

                impl GlobalEvent for #total_event_ident {
                    type WD = #world_type;
                    type GC = #total_component_ident;
                }
            },
            // total_id_struct_ident
            quote! {
                #total_id_ident
            },
            //world_multiplayer_handling
            quote! {
                Some((self.world_events.write().unwrap(), self.all_world_events.write().unwrap(), self.is_server))
            },
            quote! {
                multi_choice:HordeMultiModeChoice,
            },
            quote! {
                self.is_server
            }
        )
    }
    else {

        let mut engine_reader_writer_ident = Ident::new(format!("{}ReadWrite", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        let mut engine_reader_ident = Ident::new(format!("{}Reader", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        let mut engine_writer_ident = Ident::new(format!("{}Writer", user_data.engine_ident.to_string().trim()).trim(), OtherSpan::call_site());
        (
            quote! {},
            quote! {},
            quote! {},
            quote! {},
            quote! {},
            quote! {
                #[derive(Clone)]
                pub struct #engine_reader_writer_ident {
                    #(#ent_idents:<#ent_types as Entity<#total_id_ident>>::EV<#total_id_ident>),*,
                    #world_id:WorldHandler<#world_type, #total_id_ident>,
                }
                pub struct #engine_reader_ident <'a> {
                    #(pub #ent_idents:#entity_reader_idents<'a, #total_id_ident>),*,
                    pub #world_id:WorldComputeHandler<'a, #world_type, #total_id_ident>,
                }
                pub struct #engine_writer_ident <'a> {
                    #(pub #ent_idents:#entity_writer_idents<'a, #total_id_ident>),*,
                    pub #world_id:WorldWriteHandler<'a, #world_type, #total_id_ident>,
                }


                impl #engine_reader_writer_ident {
                    pub fn get_write<'a>(&'a mut self) -> #engine_writer_ident <'a> {
                        #engine_writer_ident {
                            #(#ent_idents:self.#ent_idents.get_write()),*,
                            #world_id:WorldWriteHandler::from_world_handler(&self.#world_id),
                        }
                    }
                    pub fn get_read<'a>(&'a self) -> #engine_reader_ident<'a> {
                        #engine_reader_ident {
                            #(#ent_idents:self.#ent_idents.get_read()),*,
                            #world_id:WorldComputeHandler::from_world_handler(&self.#world_id),    
                        }
                    }
                    pub fn get_from_engine(engine:&#engine_struct_ident) -> Self {
                        Self {
                            #(#ent_idents:engine.#ent_idents.clone()),*,
                            #world_id:engine.#world_id.clone(),
                        }
                    }
                }
            },
            quote! {},
            quote! {
                None
            },
            quote! {},
            quote! {false}
        )
    };

    let all_handlers = quote ! {
        #(&#entity_handler_names),*
    };

    let (extra_struct_addon, extra_funcs_addon, extra_new_addon,extra_func_get_addon) = match user_data.extra_data.clone() {
        Some(extra_data) => {
            let extra_data_type = user_data.extra_data_type.clone().unwrap();
            (
                quote! {pub #extra_data:#extra_data_type,},
                quote! {#extra_data:#extra_data_type,},
                quote! {#extra_data},
                quote! {&self.#extra_data}

            )
        },
        None => (
            quote! {},
            quote! {},
            quote! {},
            quote! {}
        )
    };

    let gen = quote! {

        #[derive(Eq, PartialEq, Hash, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes, Clone)]
        pub enum #total_id_ident {
            #(#ent_idents(usize)),*, // <#ent_types as MultiplayerEntity<#total_id_ident>>::ID
            #world_id
        }

        impl Identify for #total_id_ident {

        }

        #total_id_definition

        #[derive(Clone)]
        pub struct #engine_struct_ident {
            #(pub #ent_idents:<#ent_types as Entity<#total_id_ident>>::EV<#total_id_ident>),*,
            pub #world_id:WorldHandler<#world_type, #total_id_ident>,
            #rendering_struct_addon
            #multiplayer_struct_addon
            #extra_struct_addon
        }

        impl #engine_struct_ident {
            pub fn new(#(#ent_idents:<#ent_types as Entity<#total_id_ident>>::EV<#total_id_ident>),*, #world_id:WorldHandler<#world_type, #total_id_ident>, #rendering_struct_addon #multiplayer_new_addon #extra_funcs_addon) -> Self {
                #multiplayer_creator
                Self {
                    #(#ent_idents),*,
                    #world_id,
                    #rendering_type_ident
                    #multiplayer_type_ident
                    #extra_new_addon
                }
            }
            #rendering_func
            #multiplayer_func
        }

        impl GameEngine for #engine_ident {
            type GEC = #engine_struct_ident;
            type MOID = #total_id_ident;
        }

        impl MovingObjectID<#engine_ident> for #total_id_ident {
            fn get_position(&self, compute:&#engine_struct_ident) -> Vec3Df {
                match self {
                    #(#total_id_ident::#ent_idents(identity) => {
                        compute.#ent_idents.get_position_of(*identity)
                    }),*,
                    #total_id_ident::#world_id => Vec3Df::zero()
                }
            }
            fn for_each_position<I:Iterator<Item = Self>, T, D:FnMut(Self, Vec3Df) -> T>(moids:&mut I, do_func:&mut D, compute:&#engine_struct_ident) -> Vec<T> {
                // let reader = compute.#ent_idents.read().unwrap();
                let mut Ts = Vec::with_capacity(moids.size_hint().0);
                for moid in moids {
                    Ts.push(do_func(moid.clone(), moid.get_position(compute)))
                }
                Ts
            }
        }

        
        impl IndividualTask for #engine_struct_ident {
            type TID = usize;
            type TD = usize;
            fn do_task(&mut self, task_id:usize, thread_number:usize, number_of_threads:usize) {
                match task_id {
                    0 => self.apply_all_events(),
                    1 => self.main_tick(),
                    2 => self.after_main_tick(),
                    #rendering_task
                    #multiplayer_task
                    _ => panic!("No task ids after 4"),
                }
            }
        }

        struct #engine_compute_ident {
            #(#ent_idents:<<#ent_types as Entity<#total_id_ident>>::EV<#total_id_ident> as EntityVec<#total_id_ident>>::OutVec),*,
            #world_id:WorldOutHandler<#world_type, #total_id_ident>,
        }

        impl #engine_struct_ident {

            fn reset_stops(&mut self) {
                #(self.#ent_idents.reset_stop());*;
                self.#world_id.reset_stop();
                #(self.#ent_idents.stops.iteration_counter.reset());*;
            }
            
            fn apply_all_events(&mut self) {
                #(
                    {
                        self.#ent_idents.apply_all_events(#entity_multiplayer_handling)
                    }
                );*;
                self.#world_id.apply_all_events(&mut WorldWriteHandler::from_world_handler(&self.#world_id), #world_multiplayer_handling);
                self.reset_stops();
            }
            fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
                #(
                    {
                        self.#ent_idents.stops.update_number_of_threads(number_of_threads, thread_number);
                    }
                );*;
                self.#world_id.update_number_of_threads(number_of_threads, thread_number);
            }
            fn main_tick(&self) {
                #(
                    let #entity_handler_names = self.#ent_idents.get_read()
                );*;
                let #world_handler_name = WorldComputeHandler::from_world_handler(&self.#world_id);
                #(
                    let mut counter = self.#ent_idents.stops.iteration_counter.clone();
                    counter.update_len(#entity_handler_names.get_expected_len());
                    
                    counter.initialise();
                    for ent in counter {
                        // println!("{}", ent);
                        compute_tick(EntityTurn::#ent_idents, ent, #all_handlers, & #world_handler_name, #extra_func_get_addon)
                    }
                );*;
            }
            fn after_main_tick(&self) {
                #(
                    let #entity_handler_names = self.#ent_idents.get_read()
                );*;
                let #world_handler_name = WorldComputeHandler::from_world_handler(&self.#world_id);
                #(
                    let mut counter = self.#ent_idents.stops.iteration_counter.clone();
                    counter.update_len(#entity_handler_names.get_expected_len());
                    counter.initialise();
                    for ent in counter {
                        after_main_tick(EntityTurn::#ent_idents, ent, #all_handlers, & #world_handler_name, #extra_func_get_addon)
                    }
                );*;
            }
        }

        pub enum EntityTurn {
            #(#ent_idents),*,
        }

        
    };

    gen.into()
}