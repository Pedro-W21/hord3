extern crate proc_macro;

use std::sync::{Arc, RwLock};

use proc_macro::{TokenStream};
use quote::{quote, __private::Span};
use syn::{self, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields::{self, *}, FieldsNamed, FieldsUnnamed, Ident, Path, __private::TokenStream2};
#[cfg(test)]
mod tests;

#[proc_macro_derive(Entity, attributes(used_in_new, used_in_render, must_sync, position, static_id))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast:DeriveInput = syn::parse(input).unwrap();

    match &ast.data {
        Data::Struct(datastruct) => match &datastruct.fields {
            Fields::Named(fields) => derive_entity_on_struct(&ast, datastruct, fields),
            _ => panic!("Can only derive Entity on structs with named fields")
        }
        _ => panic!("Can only derive Entity on structs")
    }

}

fn get_entity_vec(ast:&DeriveInput, data:&DataStruct, fields:&FieldsNamed) -> (TokenStream2, Ident, Ident, TokenStream2) {
    let ent_ident = ast.ident.clone();

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let mut tunnel_in_components = Vec::new();
    let mut tunnel_out_components = Vec::new();
    let mut arw_types = Vec::new();
    let mut arw_components = Vec::new();
    let mut used_new_types = Vec::new();
    let mut used_new_components = Vec::new();
    let mut used_render_types = Vec::new();
    let mut used_render_components = Vec::new();
    let mut used_render_writers = Vec::new();
    let mut position_type = None;
    let mut position_component = None;
    let mut static_type_id_component = None;
    let mut static_type_id_type = None;
    let mut must_sync_types = Vec::new();
    let mut must_sync_components = Vec::new();


    for field in &fields.named {
        arw_components.push(field.ident.as_ref().unwrap().clone());
        arw_types.push(field.ty.clone());
        tunnel_in_components.push(Ident::new(format!("{}_in", field.ident.as_ref().unwrap().clone()).trim(), Span::call_site()));
        tunnel_out_components.push(Ident::new(format!("{}_out", field.ident.as_ref().unwrap().clone()).trim(), Span::call_site()));
        
        let mut used_new = false;
        let mut used_render = false;
        let mut must_sync = false;
        let mut position = false;
        let mut static_type_id = false;
        for attr in &field.attrs {
            if attr.path.is_ident(&Ident::new("used_in_new", Span::call_site())) {
                used_new = true;
            }
            if attr.path.is_ident(&Ident::new("used_in_render", Span::call_site())) {
                used_render = true;
            }
            if attr.path.is_ident(&Ident::new("must_sync", Span::call_site())) {
                must_sync = true;
            }
            if attr.path.is_ident(&Ident::new("position", Span::call_site())) {
                position = true;
            }
            if attr.path.is_ident(&Ident::new("static_id", Span::call_site())) {
                static_type_id = true;
                used_render = true;
            }
        }
        if position {
            match &position_type {
                Some(ident) => panic!("Already got position !"),
                None => {
                    position_type = Some(field.ty.clone());
                    position_component = Some(field.ident.as_ref().unwrap().clone());
                }
            }
        }
        if static_type_id {
            match &static_type_id_component {
                Some(ident) => panic!("Already got static type id"),
                None => {
                    static_type_id_component = Some(field.ident.as_ref().unwrap().clone());
                    static_type_id_type = Some(field.ty.clone());
                }
            }
        }
        if used_new {
            used_new_components.push(field.ident.as_ref().unwrap().clone());
            used_new_types.push(field.ty.clone());
        }
        if used_render {
            used_render_components.push(field.ident.as_ref().unwrap().clone());
            used_render_types.push(field.ty.clone());
            used_render_writers.push(Ident::new(format!("{}_writer", field.ident.as_ref().unwrap().to_string()).trim(), Span::call_site()))
        }
        if must_sync {
            must_sync_components.push(field.ident.as_ref().unwrap().clone());
            must_sync_types.push(field.ty.clone());
        }
    }

    let gen_vec_type = Ident::new(format!("{}Vec", ent_ident.to_string()).trim(), Span::call_site());
    let gen_vec_tunnels_in = Ident::new(format!("{}VecTunnelsIn", ent_ident.to_string()).trim(), Span::call_site());
    let gen_vec_tunnels_out = Ident::new(format!("{}VecTunnelsOut", ent_ident.to_string()).trim(), Span::call_site());
    let gen_vec_read_type = Ident::new(format!("{}VecRead", ent_ident.to_string()).trim(), Span::call_site());
    let gen_vec_write_type = Ident::new(format!("{}VecWrite", ent_ident.to_string()).trim(), Span::call_site());
    let gen_vec_out_type = Ident::new(format!("{}VecOut", ent_ident.to_string()).trim(), Span::call_site());
    let gen_new_ent_type = Ident::new(format!("New{}", ent_ident.to_string()).trim(), Span::call_site());
    let gen_render_ent_type = Ident::new(format!("Render{}", ent_ident.to_string()).trim(), Span::call_site());
    let mut event_types = Vec::new();

    let sync_component_enum_id = Ident::new(format!("{}SyncComponent", ent_ident.to_string()).trim(), Span::call_site());
    let static_type_ident = Ident::new(format!("Static{}", ent_ident.to_string()).trim(), Span::call_site());
    let static_type_id_component = static_type_id_component.unwrap();
    let position_ident = position_component.unwrap();
    let position_type = position_type.unwrap();
    let (vec_type, new_and_apply_events, new_ent_type_generics) = if must_sync_types.len() > 0 {
        let sync_event_type_id = Ident::new(format!("{}SyncEvent", ent_ident.to_string()).trim(), Span::call_site());
        let sync_event_enum_id = Ident::new(format!("{}SyncEventVariant", ent_ident.to_string()).trim(), Span::call_site());
        let event_type_id = Ident::new(format!("{}Event", ent_ident.to_string()).trim(), Span::call_site());
        for field in &fields.named {
            let field_type = field.ty.clone();
            event_types.push(quote! {
                #event_type_id<<#field_type as Component<ID>>::CE>
            });
        }
        
        (
            quote! {



                impl<TID:Identify> MultiplayerEntity<TID> for #ent_ident {
                    type ID = usize;
                    type GEV<O> = #sync_event_enum_id<TID>;
                    type GEC = #sync_component_enum_id;
                }
                
                #[derive(Clone, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes, PartialEq)]
                pub struct #gen_new_ent_type<ID:Identify> {
                    #(#used_new_components:#used_new_types),* ,
                    must_be_synced:bool,
                    created_by:Option<ID>
                }

                impl<ID:Identify> #gen_new_ent_type<ID> {
                    pub fn new(#(#used_new_components:#used_new_types),*, must_be_synced:bool, created_by:Option<ID>) -> Self {
                        Self {
                            #(#used_new_components),* ,
                            must_be_synced,
                            created_by
                        }
                    }
                    pub fn get_static_type_id(&self) -> usize {
                        <#static_type_id_type as HasStaticTypeID>::get_id(&self.#static_type_id_component)
                    }
                }
                #[derive(Clone, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes, PartialEq)]
                pub enum #sync_event_enum_id<ID:Identify> {
                    //CoolComponent(usize),
                    #(#arw_components (<#arw_types as Component<ID>>::CE)),*,

                    NewEnt { ent:#gen_new_ent_type<ID>, new_id:usize, made_by:Option<ID> }
                }

                impl<ID:Identify> #sync_event_enum_id<ID> {
                    pub fn get_source(&self) -> Option<ID> {
                        match &self {
                            #(#sync_event_enum_id::#arw_components(evt) => evt.get_source()),*,
                            #sync_event_enum_id::NewEnt {made_by, ..} => made_by.clone()
                        }
                    }
                    pub fn get_id(&self) -> EntityID {
                        match &self {
                            #(#sync_event_enum_id::#arw_components(evt) => <<#arw_types as Component<ID>>::CE as ComponentEvent<#arw_types, ID>>::get_id(evt)),*,
                            #sync_event_enum_id::NewEnt {new_id, ..} => new_id.clone()
                        }
                    }
                }

                pub struct #event_type_id<T> {
                    pub must_be_synced:bool,
                    pub event:T,
                }

                impl<T> #event_type_id<T> {
                    pub fn new(must_be_synced:bool, event:T) -> Self {
                        Self {must_be_synced, event}
                    }
                }

                #[derive(Clone)]
                pub struct #gen_vec_type<ID:Identify> {
                    #(pub #arw_components:std::sync::Arc<std::sync::RwLock<Vec<#arw_types>>>),* ,
                    pub static_types:std::sync::Arc<std::sync::RwLock<Vec<#static_type_ident<ID>>>>,
                    pub tunnels_in:#gen_vec_tunnels_in<ID>,
                    pub tunnels_out:#gen_vec_tunnels_out<ID>,
                    pub available_entities:std::sync::Arc<std::sync::RwLock<std::collections::VecDeque<EntityID>>>,
                    pub stops:EVecStopsIn,
                    pub to_sync:std::sync::Arc<std::sync::RwLock<Vec<#sync_event_enum_id<ID>>>>,
                    pub all_events:std::sync::Arc<std::sync::RwLock<Vec<#sync_event_enum_id<ID>>>>
                }
            },
            quote! {
                pub fn new(capacity:usize) -> Self {
                    #(let #arw_components = std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(capacity))));* ;
                    let static_types = std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(capacity/10)));
                    let (tunnels_in, tunnels_out) = #gen_vec_tunnels_in::new();
                    let available_entities = std::sync::Arc::new(std::sync::RwLock::new(std::collections::VecDeque::with_capacity(capacity)));
                    let (stops_in, stops_out) = EVecStopsIn::new();
                    let to_sync = std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(2048)));
                    Self {
                        #(#arw_components:#arw_components.clone()),* ,
                        static_types:static_types.clone(),
                        tunnels_in,
                        tunnels_out,
                        available_entities:available_entities.clone(),
                        stops:stops_in,
                        to_sync,
                        all_events:std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(2048))),
                    }
                }
                pub fn apply_all_events<'a>(&'a self, is_server:bool) {
                    let mut write_handler = self.get_write();
                    let mut to_sync_write = self.to_sync.write().unwrap();
                    if is_server {
                        let mut all_events_write = self.all_events.write().unwrap();
                        #(
                            {
                                while let Ok(event) = self.tunnels_in.#tunnel_in_components.recv_timeout(std::time::Duration::from_nanos(10)) {
                                    if event.must_be_synced {
                                        to_sync_write.push(#sync_event_enum_id::#arw_components(event.event.clone()));
                                    }
                                    all_events_write.push(#sync_event_enum_id::#arw_components(event.event.clone()));
                                    <<#arw_types as Component<ID>>::CE as ComponentEvent<#arw_types, ID>>::apply_to_component(event.event.clone(), &mut write_handler.#arw_components);
                                }
                            }
                        );* ;
                        {
                            while let Ok(ent) = self.tunnels_in.new_ents.recv_timeout(std::time::Duration::from_nanos(10)) {
                                let new_id = write_handler.new_ent(ent.clone());
                                all_events_write.push(#sync_event_enum_id::NewEnt{made_by: ent.created_by.clone(), ent:ent.clone(), new_id});
                                if ent.must_be_synced {
                                    
                                    to_sync_write.push(#sync_event_enum_id::NewEnt{made_by: ent.created_by.clone(), ent, new_id});
                                }
                            }
                        }
                    }
                    else {
                        #(
                            {
                                while let Ok(event) = self.tunnels_in.#tunnel_in_components.recv_timeout(std::time::Duration::from_nanos(10)) {
                                    if event.must_be_synced {
                                        to_sync_write.push(#sync_event_enum_id::#arw_components(event.event.clone()));
                                    }
                                    <<#arw_types as Component<ID>>::CE as ComponentEvent<#arw_types, ID>>::apply_to_component(event.event.clone(), &mut write_handler.#arw_components);
                                }
                            }
                        );* ;
                        {
                            while let Ok(ent) = self.tunnels_in.new_ents.recv_timeout(std::time::Duration::from_nanos(10)) {
                                if ent.must_be_synced {
                                    let new_id = write_handler.new_ent(ent.clone());
                                    to_sync_write.push(#sync_event_enum_id::NewEnt{made_by: ent.created_by.clone(), ent, new_id});
                                }
                                else {
                                    write_handler.new_ent(ent);
                                }
                            }
                        }
                    }
                    
                }
                pub fn change_component<'a>(&'a self, component:#sync_component_enum_id, id:usize) {
                    let mut write_handler = self.get_write();
                    match component {
                        #(#sync_component_enum_id::#arw_components (data) => {write_handler.#arw_components[id] = data}),*,
                    }
                }
                pub fn is_that_component_correct(&self, component:#sync_component_enum_id, id:usize) -> bool {
                    let mut read_handler = self.get_read();
                    match component {
                        #(#sync_component_enum_id::#arw_components (data) => {read_handler.#arw_components[id] == data}),*,
                    }
                }
                pub fn apply_one_event<'a>(&'a self, event:#sync_event_enum_id<ID>) {
                    let mut write_handler = self.get_write();
                    match event {
                        #(#sync_event_enum_id::#arw_components (sub_event) => <<#arw_types as Component<ID>>::CE as ComponentEvent<#arw_types, ID>>::apply_to_component(sub_event, &mut write_handler.#arw_components)),*,
                        #sync_event_enum_id::NewEnt{ent, new_id, made_by} => {write_handler.new_ent(ent);}, 
                    }
                }
                pub fn get_need_sync<'a>(&'a self) -> std::sync::RwLockWriteGuard<Vec<#sync_event_enum_id<ID>>> {
                    self.to_sync.write().unwrap()
                }
                pub fn get_all_events<'a>(&'a self) -> std::sync::RwLockWriteGuard<Vec<#sync_event_enum_id<ID>>> {
                    self.all_events.write().unwrap()
                }
            },
            quote! {
                <ID>
            }
        )
    }
    else {
        for field in &fields.named {
            let field_type = field.ty.clone();
            event_types.push(quote! {
                <#field_type as Component<ID>>::CE
            });
        }
        (
            quote! {
                #[derive(Clone)]
                pub struct #gen_new_ent_type {
                    #(#used_new_components:#used_new_types),* ,
                }

                impl #gen_new_ent_type {
                    pub fn new(#(#used_new_components:#used_new_types),*) -> Self {
                        Self {
                            #(#used_new_components),* ,
                        }
                    }
                    pub fn get_static_type_id(&self) -> usize {
                        <#static_type_id_type as HasStaticTypeID>::get_id(&self.#static_type_id_component)
                    }
                }

                #[derive(Clone)]
                pub struct #gen_vec_type<ID:Identify> {
                    #(pub #arw_components:std::sync::Arc<std::sync::RwLock<Vec<#arw_types>>>),* ,
                    pub static_types:std::sync::Arc<std::sync::RwLock<Vec<#static_type_ident<ID>>>>,
                    pub tunnels_in:#gen_vec_tunnels_in<ID>,
                    pub tunnels_out:#gen_vec_tunnels_out<ID>,
                    pub available_entities:std::sync::Arc<std::sync::RwLock<std::collections::VecDeque<EntityID>>>,
                    pub stops:EVecStopsIn
                }
            },
            quote! {
                pub fn new(capacity:usize) -> Self {
                    #(let #arw_components = std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(capacity))));* ;
                    let static_types = std::sync::Arc::new(std::sync::RwLock::new(Vec::with_capacity(capacity/10)));
                    let (tunnels_in, tunnels_out) = #gen_vec_tunnels_in::new();
                    let available_entities = std::sync::Arc::new(std::sync::RwLock::new(std::collections::VecDeque::with_capacity(capacity)));
                    let (stops_in, stops_out) = EVecStopsIn::new();
                    Self {
                        #(#arw_components:#arw_components.clone()),* ,
                        static_types:static_types.clone(),
                        tunnels_in,
                        tunnels_out,
                        available_entities:available_entities.clone(),
                        stops:stops_in
                    }
                }
                pub fn apply_all_events<'a>(&'a self, useless_bool:bool) { // a bit ugly but cleaner than the alternative I think
                    let mut write_handler = self.get_write();
                    #(
                        {
                            while let Ok(event) = self.tunnels_in.#tunnel_in_components.recv_timeout(std::time::Duration::from_nanos(10)) {
                                <<#arw_types as Component<ID>>::CE as ComponentEvent<#arw_types, ID>>::apply_to_component(event, &mut write_handler.#arw_components);
                            }
                        }
                    );* ;
                    {
                        while let Ok(ent) = self.tunnels_in.new_ents.recv_timeout(std::time::Duration::from_nanos(10)) {
                            write_handler.new_ent(ent);
                        }
                    }
                }
            },
            quote! {

            }
        )
    };
   
    

    let render_part = if used_render_components.len() > 0 {
        let first_component = used_render_components[0].clone();
        quote! {
            pub trait #gen_render_ent_type<RB, ID:Identify> {
                fn do_render_changes(rendering_data:&mut RB, #(#used_render_components:&mut #used_render_types),*, static_type:&#static_type_ident<ID>);
            }

            impl<ID:Identify> #gen_vec_type<ID> {
                pub fn do_all_renders<RB>(&mut self, rendering_data:&mut RB) where #ent_ident:#gen_render_ent_type<RB, ID> {
                    #(let mut #used_render_components = self.#used_render_components.write().unwrap());*;
                    let static_types = self.static_types.read().unwrap();
                    let len = #first_component.len();
                    for i in 0..len {
                        let static_type = <#static_type_id_type as HasStaticTypeID>::get_id(&#static_type_id_component[i]);
                        <#ent_ident as #gen_render_ent_type<RB, ID>>::do_render_changes(rendering_data, #(&mut #used_render_components[i]),*, &static_types[static_type]);
                    }
                }
            }
        }
    }
    else {
        quote! {}
    };
    let first_component = &arw_components[0].clone();
    let mut gen_vec = quote! {
        #render_part

        #[derive(Clone, to_from_bytes_derive::ToBytes, to_from_bytes_derive::FromBytes, PartialEq)]
        pub enum #sync_component_enum_id {
            #(#arw_components (#arw_types)),*,
        }
        
        #[derive(Clone)]
        pub struct #gen_vec_tunnels_in<ID:Identify> {
            #(pub #tunnel_in_components:std::sync::mpmc::Receiver<#event_types>),* ,
            pub new_ents:std::sync::mpmc::Receiver<#gen_new_ent_type #new_ent_type_generics>,
        }

        impl<ID:Identify> #gen_vec_tunnels_in<ID> {
            pub fn new() -> (Self, #gen_vec_tunnels_out <ID>) {
                #(
                let (#tunnel_out_components, #tunnel_in_components) = std::sync::mpmc::channel()
                );* ;
                let (new_ents_out, new_ents_in) = std::sync::mpmc::channel();
                (
                    #gen_vec_tunnels_in {
                        #(#tunnel_in_components),* ,
                        new_ents:new_ents_in,
                    },
                    #gen_vec_tunnels_out {
                        #(#tunnel_out_components),* ,
                        new_ents:new_ents_out
                    }
                )
            }
        }

        #[derive(Clone)]
        pub struct #gen_vec_tunnels_out<ID:Identify> {
            #(pub #tunnel_out_components:std::sync::mpmc::Sender<#event_types>),* ,
            pub new_ents:std::sync::mpmc::Sender<#gen_new_ent_type #new_ent_type_generics>,
        }
        
        #vec_type

        impl<ID:Identify> #gen_vec_type<ID> {
            #new_and_apply_events
            pub fn get_write<'a>(&'a self) -> #gen_vec_write_type <'a, ID> {
                #gen_vec_write_type {
                    #(#arw_components:self.#arw_components.write().unwrap()),* ,
                    static_types:self.static_types.write().unwrap(),
                    available_entities:self.available_entities.write().unwrap(),
                }
            }

            pub fn reset_stop(&mut self) {
                self.stops.reset_stop(None)
            }

            pub fn get_position_of(&self, id:usize) -> Vec3Df {
                let reader = self.#position_ident.read().unwrap();
                <#position_type as EntityPosition<ID>>::get_pos(&reader[id])
            }

            pub fn update_number_of_threads(&mut self, number_of_threads:usize) {
                self.stops.reset_stop(Some(number_of_threads))
            }
            pub fn get_read<'a>(&'a self) -> #gen_vec_read_type <'a, ID> {
                #gen_vec_read_type {
                    #(#arw_components:self.#arw_components.read().unwrap()),* ,
                    static_types:self.static_types.read().unwrap(),
                    tunnels:self.tunnels_out.clone()
                }
            }
        }
        pub struct #gen_vec_read_type <'a, ID:Identify> {
            #(pub #arw_components:std::sync::RwLockReadGuard<'a, Vec<#arw_types>>),* ,
            pub static_types:std::sync::RwLockReadGuard<'a, Vec<#static_type_ident<ID>>>,
            pub tunnels:#gen_vec_tunnels_out<ID>,
        }

        impl<'a, ID:Identify> #gen_vec_read_type <'a, ID> {
            pub fn get_components_for(&'a self, id:usize) -> Vec<#sync_component_enum_id> {
                let mut components = Vec::with_capacity(8); //Estimate could be exact
                #(components.push(#sync_component_enum_id::#must_sync_components(self.#must_sync_components[id].clone())));*;
                components
            }
            pub fn get_expected_len(&'a self) -> usize {
                self.#first_component.len()
            }
        }

        pub struct #gen_vec_write_type <'a, ID:Identify> {
            #(pub #arw_components:std::sync::RwLockWriteGuard<'a, Vec<#arw_types>>),* ,
            pub available_entities:std::sync::RwLockWriteGuard<'a, std::collections::VecDeque<usize>>,
            pub static_types:std::sync::RwLockWriteGuard<'a, Vec<#static_type_ident<ID>>>
        }

        impl<'a, ID:Identify> #gen_vec_write_type <'a, ID> {
            pub fn new_ent(&mut self, new_ent:#gen_new_ent_type #new_ent_type_generics) -> usize {
                let static_type = new_ent.get_static_type_id();
                let ent = <#gen_new_ent_type #new_ent_type_generics as NewEntity<#ent_ident #ty_generics, ID>>::get_ent(new_ent, &self.static_types[static_type]);
                match self.available_entities.pop_back() {
                    Some(id) => {
                        #(self.#arw_components[id] = ent.#arw_components);*;
                        id
                    }
                    None => {
                        let id = self.#first_component.len();
                        #(self.#arw_components.push(ent.#arw_components));*;
                        id
                    }
                }
            }
            pub fn new_sct(&mut self, sct:#static_type_ident<ID>) {
                self.static_types.push(sct);
            }
        } 
        #[derive(Clone)]
        pub struct #gen_vec_out_type<ID:Identify> {
            #(pub #arw_components:std::sync::Arc<std::sync::RwLock<Vec<#arw_types>>>),* ,
            pub static_types:std::sync::Arc<std::sync::RwLock<Vec<#static_type_ident<ID>>>>,
            pub tunnels_out:#gen_vec_tunnels_out<ID>,
            pub available_entities:std::sync::Arc<std::sync::RwLock<std::collections::VecDeque<EntityID>>>,
            pub stops:EVecStopsOut
        }

        impl<ID:Identify> #gen_vec_out_type<ID> {
            pub fn update_number_of_threads(&mut self, number_of_threads:usize, thread_number:usize) {
                self.stops.update_number_of_threads(number_of_threads, thread_number)
            }
            pub fn get_read<'a>(&'a self) -> #gen_vec_read_type <'a, ID> {
                #gen_vec_read_type {
                    #(#arw_components:self.#arw_components.read().unwrap()),* ,
                    static_types:self.static_types.read().unwrap(),
                    tunnels:self.tunnels_out.clone()
                }
            }
        }


        impl<ID:Identify> EntityVec<ID> for #gen_vec_type<ID> {
            type OutVec = #gen_vec_out_type<ID>;
        }
    };
    
    (gen_vec, gen_vec_type, gen_new_ent_type, new_ent_type_generics)
}

fn get_static_entity(ast:&DeriveInput, data:&DataStruct, fields:&FieldsNamed) -> (TokenStream2, Ident) {
    let ent_ident = ast.ident.clone();

    let mut arw_types = Vec::new();
    let mut arw_components = Vec::new();


    for field in &fields.named {
        arw_components.push(field.ident.as_ref().unwrap().clone());
        arw_types.push(field.ty.clone());
    }

    let static_type_ident = Ident::new(format!("Static{}", ent_ident.to_string()).trim(), Span::call_site());

    let mut gen = quote! {
        pub struct #static_type_ident<ID:Identify> {
            #(pub #arw_components:<#arw_types as Component<ID>>::SC),*
        }

        impl<ID:Identify> StaticEntity<ID> for #static_type_ident<ID> {

        }
    };

    (gen, static_type_ident)
}

fn derive_entity_on_struct(ast:&DeriveInput, data:&DataStruct, fields:&FieldsNamed) -> TokenStream {
    let name = &ast.ident.clone();

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    
    let (ent_vec_create, ent_vec_type, new_ent_type, new_ent_type_generics) = get_entity_vec(ast, data, fields);

    let (static_ent_create, static_ent_type) = get_static_entity(ast, data, fields);

    let mut gen = quote! {
        #static_ent_create

        #ent_vec_create

        impl<ID:Identify> #impl_generics Entity<ID> for #name #ty_generics {
            type EV<O> = #ent_vec_type<ID>;
            type SE = #static_ent_type<ID>;
            type NE = #new_ent_type #new_ent_type_generics;
        }

    };

    gen.into()
}

fn used_in_new(args:TokenStream, input:TokenStream) -> TokenStream {
    input
}