extern crate proc_macro;

use std::{collections::{HashMap, HashSet}, sync::{Arc, RwLock}};

use proc_macro::{Span, TokenStream};
use quote::{quote, __private::Span as OtherSpan};
use syn::{self, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields::{self, *}, FieldsNamed, FieldsUnnamed, Ident, Path, Type, __private::TokenStream2, punctuated::Punctuated, Lit, Meta};

#[proc_macro_derive(HordeTask, attributes(uses_type, max_threads, type_task_id, uses_generic))] 
pub fn derive_task(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast:DeriveInput = syn::parse(input).unwrap();

    match &ast.data {
        Data::Enum(dataenum) => derive_task_on_enum(&ast, dataenum),
        _ => panic!("Can only derive HordeTask on enums")
    }

}

fn derive_task_on_enum(ast:&DeriveInput, dataenum:&DataEnum) -> TokenStream {

    let mut types_needed = Vec::new();
    let mut max_threads_per_task = Vec::new();
    let mut task_id_for_task = Vec::new();
    let mut unique_types = Vec::new();
    let mut unique_type_names = Vec::new();
    let mut generics_combinations:HashMap<Option<String>, Vec<Ident>> = HashMap::new();
    let mut generics_per_type = Vec::new();
    let mut generics_per_unique_type = Vec::new();

    let mut all_variants = Vec::new();
    let mut unique_type_strs = Vec::new();

    let enum_id = ast.ident.clone();
    
    let mut genericced_types = HashSet::new();

    for task in &dataenum.variants {
        all_variants.push(task.ident.clone());

        let mut type_used = None;
        for attribute in &task.attrs {
            match attribute.parse_meta() {
                Ok(meta) => match meta {
                    Meta::NameValue(name_value) => {
                        match name_value.path.get_ident() {
                            Some(ident) => {
                                let id = ident.to_string();
                                let str_ident = id.trim();
                                match str_ident {
                                    "uses_type" => match name_value.lit {
                                        Lit::Str(value) => {
                                            let val = value.value();
                                            types_needed.push(Ident::new(val.to_lowercase().trim(), OtherSpan::call_site()));
                                            type_used = Some(val.clone());
                                            if !unique_type_strs.contains(&val) {
                                                unique_types.push(Ident::new(val.clone().trim(), OtherSpan::call_site()));
                                                unique_type_names.push(Ident::new(val.to_lowercase().trim(), OtherSpan::call_site()));
                                                unique_type_strs.push(val);
                                            }
                                        },
                                        _ => panic!("Type name in Meta not a string")
                                    },
                                    "max_threads" => match name_value.lit {
                                        Lit::Int(int_value) => {
                                            let val = int_value.base10_parse::<usize>().expect("Not a valid number of threads to use");
                                            max_threads_per_task.push(val);
                                        }
                                        _ => panic!("number of threads in Meta not a positive integer")
                                    },
                                    "type_task_id" => match name_value.lit {
                                        Lit::Int(int_value) => {
                                            let val = int_value.base10_parse::<usize>().expect("Not a valid task id number");
                                            task_id_for_task.push(val);
                                        }
                                        _ => panic!("task_id in Meta not a positive integer")
                                    },
                                    "uses_generic" => match name_value.lit {
                                        Lit::Str(value) => {
                                            let val = value.value();
                                            let gen_ident = Ident::new(val.clone().trim(), OtherSpan::call_site());
                                            match generics_combinations.get_mut(&type_used) {
                                                Some(vector) => vector.push(gen_ident),
                                                None => {generics_combinations.insert(type_used.clone(), vec![gen_ident]);}
                                            }
                                        },
                                        _ => panic!("generic in uses_generic mut be in string literal form")
                                    },
                                    _ => panic!("Meta name = value for {} enum attribute has unknown name {};", task.ident.to_string(), str_ident)
                                }
                            },
                            None => panic!("Meta name = value for {} enum attribute doesn't have simple name path", task.ident.to_string())
                        }
                    },
                    _ => panic!("Meta for {} enum attribute isn't name = value pair", task.ident.to_string())
                },
                Err(error) => panic!("Error while parsing {} attributes : {}", task.ident.to_string(),error)
            }
        }
        match &type_used {
            Some(named) => {
                if !genericced_types.contains(named) {
                    genericced_types.insert(named.clone());
                    match generics_combinations.get(&None) {
                        Some(vector) => {
                            generics_per_type.push(quote! {<#(#vector),*>});
                            if named.clone() == unique_types.last().unwrap().to_string() {
                                generics_per_unique_type.push(quote! {<#(#vector),*>});
                            }
                        },
                        None => match generics_combinations.get(&type_used) {
                            Some(vector) => {
                                
                                generics_per_type.push(quote! {<#(#vector),*>});
                                if named.clone() == unique_types.last().unwrap().to_string() {
                                    generics_per_unique_type.push(quote! {<#(#vector),*>});
                                    //panic!("{} {} {} {}", named.clone(), generics_per_unique_type.last().unwrap(), generics_per_unique_type.len(), unique_types.len());
                                }
                                
                            },
                            None => {
                                generics_per_type.push(quote! {}); 
                                if named.clone() == unique_types.last().unwrap().to_string() {
                                    generics_per_unique_type.push(quote! {});
                                }
                            },
                        }
                    }
                }
                
            },
            None => {panic!("Type used never named");},
        }
    }
    let handler_type = Ident::new(format!("{}TaskHandler", ast.ident.to_string().trim()).trim(), OtherSpan::call_site());
    let gen = quote! {
        //#(#generics_per_unique_type),*,
        #[derive(Clone)]
        pub struct #handler_type {
            #(#unique_type_names:#unique_types #generics_per_unique_type),*,
        }

        impl #handler_type {
            pub fn new(#(#unique_type_names:#unique_types #generics_per_unique_type),*) -> Self {
                Self {
                    #(#unique_type_names),*,
                }
            }
        }

        impl HordeTask for #enum_id {
            type HTH = #handler_type;
            type HTD = #handler_type;
            fn max_threads(&self) -> usize {
                match self {
                    #(&#enum_id::#all_variants => #max_threads_per_task),*,
                }
            }
            fn data_from_handler(handler:&Self::HTH) -> Self::HTD {
                handler.clone()
            }
        }

        impl HordeTaskData<#enum_id> for #handler_type {
            fn do_task(&mut self, task:#enum_id, thread_number:usize, number_of_threads:usize) {
                match task {
                    #(#enum_id::#all_variants => self.#types_needed.do_task(#task_id_for_task, thread_number, number_of_threads)),*,
                }
            }
        }

        impl HordeTaskHandler for #handler_type {

        }
    };
    gen.into()
}