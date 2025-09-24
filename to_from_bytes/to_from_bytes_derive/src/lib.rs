extern crate proc_macro;

use proc_macro::{TokenStream};
use quote::{quote, __private::Span};
use syn::{self, Data, Fields::*, Ident, FieldsNamed, FieldsUnnamed, DataStruct, DataEnum};

#[proc_macro_derive(ToBytes)]
pub fn addbytes_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_addbytes(&ast)
}

fn addbytes_unnamed_struct(ast: &syn::DeriveInput, fields_struct:&FieldsUnnamed) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut i = 0;
    let mut attrs_size = Vec::new();
    let mut attrs_names = Vec::new();
    for field in fields_struct.unnamed.iter() {
        attrs_names.push(syn::Index::from(i));
        i += 1;
    }
    i = 0;
    for field in fields_struct.unnamed.iter() {
        let index_name = &attrs_names[i];
        attrs_size.push(quote! {
            self.#index_name.get_bytes_size()
        });
        i += 1;
    }
    
    let gen = quote! {
        impl #impl_generics to_from_bytes::ToBytes for #name #ty_generics {
            fn get_bytes_size(&self) -> usize {
                #(#attrs_size)+*
            }

            fn add_bytes(&self, bytes:&mut Vec<u8>) {
                #(self.#attrs_names.add_bytes(bytes));*
            }
        }
    };
    gen.into()

}

fn addbytes_named_struct(ast: &syn::DeriveInput, fields_struct:&FieldsNamed) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    
    let mut attrs_size = Vec::new();
    for field in fields_struct.named.iter() {
        let attr_name = field.ident.as_ref().unwrap();
        attrs_size.push(quote! {
            self.#attr_name.get_bytes_size()
        })
    }
    let mut attrs_names = Vec::new();
    for field in fields_struct.named.iter() {
        let attr_name = field.ident.as_ref().unwrap();
        attrs_names.push(quote! {
            #attr_name
        })
    }
    let gen = quote! {
        impl #impl_generics to_from_bytes::ToBytes for #name #ty_generics {
            fn get_bytes_size(&self) -> usize {
                #(#attrs_size)+*
            }

            fn add_bytes(&self, bytes:&mut Vec<u8>) {
                #(self.#attrs_names.add_bytes(bytes));*
            }
        }
    };
    gen.into()

}

fn addbytes_struct(ast: &syn::DeriveInput, data:&DataStruct) -> TokenStream {
    match &data.fields {
        Named(fields_named) => addbytes_named_struct(ast, fields_named),
        Unnamed(fields_unnamed) => addbytes_unnamed_struct(ast, fields_unnamed),
        _ => panic!("ffdfs")  
    }
}

fn addbytes_named_enum(ast:&syn::DeriveInput, fields_named:&FieldsNamed, ident:&Ident, variant_number:u8) -> (TokenStream, TokenStream) {
    let mut field_names = Vec::new();
    for field in fields_named.named.iter() {
        let field_name = field.ident.as_ref().unwrap();
        field_names.push(quote!{
            #field_name
        })
    }
    (
        quote!{
            Self::#ident{#(#field_names),*} => 1 + #(#field_names.get_bytes_size())+*
        }.into(),
        quote! {
            Self::#ident{#(#field_names),*} => {
                bytes.push(#variant_number);
                #(#field_names.add_bytes(bytes));*
            }
        }.into()
    )
}

fn addbytes_unnamed_enum(ast:&syn::DeriveInput, fields_unnamed:&FieldsUnnamed, ident:&Ident, variant_number:u8) -> (TokenStream, TokenStream) {
    let mut field_names = Vec::new();
    let mut i = 0;
    for field in fields_unnamed.unnamed.iter() {
        let field_name = Ident::new(format!("field_{}", i).trim(), Span::call_site());
        field_names.push(quote!{
            #field_name
        });
        i += 1;
    }
    (
        quote!{
            Self::#ident(#(#field_names),*) => 1 + #(#field_names.get_bytes_size())+*
        }.into(),
        quote! {
            Self::#ident(#(#field_names),*) => {
                bytes.push(#variant_number);
                #(#field_names.add_bytes(bytes));*
            }
        }.into()
    )
}

fn addbytes_enum(ast:&syn::DeriveInput, data:&DataEnum) -> TokenStream {
    let mut sizes = Vec::new();
    let mut adds = Vec::new();

    let mut variant_number = 0_usize;
    for variant in &data.variants {
        let ident = &variant.ident;
        match &variant.fields {
            Named(fields_named) => {
                let (size, add) = addbytes_named_enum(ast, fields_named, ident, variant_number as u8);
                sizes.push(size.into());
                adds.push(add.into());
            },
            Unnamed(fields_unnamed) => {
                let (size, add) = addbytes_unnamed_enum(ast, fields_unnamed, ident, variant_number as u8);
                sizes.push(size.into());
                adds.push(add.into());
            },
            Unit => {
                let variant_u8 = variant_number as u8;
                
                sizes.push(quote!{
                    Self::#ident => 1
                });
                adds.push(quote!{
                    Self::#ident => bytes.push(#variant_u8)
                });
            }
        }
        variant_number += 1;
    }

    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    

    let gen = quote! {
        impl #impl_generics to_from_bytes::ToBytes for #name #ty_generics {
            fn get_bytes_size(&self) -> usize {
                match &self {
                    #(#sizes),*
                }
            }
            fn add_bytes(&self, bytes:&mut Vec<u8>) {
                match &self {
                    #(#adds),*
                }
            } 
        }
    };
    gen.into()
}



fn impl_addbytes(ast: &syn::DeriveInput) -> TokenStream {
    
    let attrs = &ast.attrs;

    match &ast.data {
      Data::Struct(datastruct) => addbytes_struct(ast, datastruct),
      Data::Enum(dataenum) => addbytes_enum(ast, dataenum),
      _ => panic!("eeaeae")
    }
    
}

#[proc_macro_derive(FromBytes)]
pub fn decodebytes_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_decodebytes(&ast)
}

fn decodebytes_named_struct(ast: &syn::DeriveInput, fields_struct:&FieldsNamed) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut attrs_names = Vec::new();
    let mut decode_field_idents = Vec::new();
    let mut counter_steps = Vec::new();
    let mut i :u8= 0;
    let mut field_types = Vec::new();
    
    let mut decode_field_types = Vec::new();
    for field in fields_struct.named.iter() {
        let attr_name = field.ident.as_ref().unwrap();
        attrs_names.push(quote! {
            #attr_name
        });
        let decode_ident = &Ident::new(format!("{}_decoder", attr_name.to_string()).trim(), Span::call_site());
        decode_field_idents.push(quote! {
            #decode_ident
        });

        counter_steps.push(quote! {
            #i
        });

        let decode_type = &field.ty;
        decode_field_types.push(quote! {
            <#decode_type as to_from_bytes::FromBytes>::Decoder
        });

        let field_type = &field.ty;
        field_types.push(quote!{
            #field_type
        });

        i += 1;
        
    }

    let struct_ident = Ident::new(format!("{}Decoder", name.to_string()).trim(), Span::call_site());
    let last_count = quote!{ #i };

    let gen = quote! {
        #[derive(Clone)]
        pub struct #struct_ident #impl_generics {
            counter:u8,
            #(#decode_field_idents:#decode_field_types),* ,
            #(#attrs_names:Option<#field_types>),*
        }

        impl #impl_generics to_from_bytes::ByteDecoder<#name #ty_generics> for #struct_ident #ty_generics {
            fn decode_byte(&mut self, bytes:&mut Vec<u8>, byte:u8) -> Option<#name #ty_generics> {
                match self.counter {
                    #(#counter_steps => {
                        match <#decode_field_types as to_from_bytes::ByteDecoder<#field_types>>::decode_byte(&mut self.#decode_field_idents, bytes, byte) {
                            Some(val) => {
                                self.#attrs_names = Some(val);
                                self.counter += 1;
                                bytes.clear();
                            },
                            None => ()
                        };
                    }),*
                    
                    _ => ()
                }
                if self.counter == #last_count {
                    bytes.clear();
                    Some(#name {
                        #(#attrs_names:self.#attrs_names.as_ref().unwrap().clone()),*
                    })
                }
                else {
                    None
                }
                
            }
            fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(#name #ty_generics, usize)> {
                let mut i = 0;
                while self.counter < #last_count && i < slice_to_decode.len() {
                    match self.counter {
                        #(#counter_steps => {
                            match <#decode_field_types as to_from_bytes::ByteDecoder<#field_types>>::decode_slice_borrow(&mut self.#decode_field_idents, bytes, &slice_to_decode[i..]) {
                                Some((val,bytes_read)) => {
                                    self.#attrs_names = Some(val);
                                    self.counter += 1;
                                    i += bytes_read;
                                    bytes.clear();
                                },
                                None => i = slice_to_decode.len()
                            };
                        }),*
                        
                        _ => ()
                    }
                    
                }
                if self.counter == #last_count {
                    bytes.clear();
                    Some((#name {
                        #(#attrs_names:self.#attrs_names.as_ref().unwrap().clone()),*
                    }, i))
                }
                else {
                    None
                }
            }
        }
        

        impl #impl_generics to_from_bytes::FromBytes for #name #ty_generics {
            type Decoder = #struct_ident #ty_generics;
            fn get_decoder() -> Self::Decoder {
                #struct_ident {
                    counter:0,
                    #(#decode_field_idents:<#field_types as to_from_bytes::FromBytes>::get_decoder()),* ,
                    #(#attrs_names:None),*
                }
            }
        }
    };
    gen.into()
}

fn decodebytes_unnamed_struct(ast: &syn::DeriveInput, fields_struct:&FieldsUnnamed) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut attrs_names = Vec::new();
    let mut decode_field_idents = Vec::new();
    let mut counter_steps = Vec::new();
    let mut i :u8= 0;
    let mut field_types = Vec::new();
    
    let mut decode_field_types = Vec::new();
    for field in fields_struct.unnamed.iter() {
        let attr_name = Ident::new(format!("ident_{}", i).trim(), Span::call_site());
        attrs_names.push(quote! {
            #attr_name
        });
        let decode_ident = &Ident::new(format!("{}_decoder", attr_name.to_string()).trim(), Span::call_site());
        decode_field_idents.push(quote! {
            #decode_ident
        });

        counter_steps.push(quote! {
            #i
        });

        let decode_type = &field.ty;
        decode_field_types.push(quote! {
            <#decode_type as to_from_bytes::FromBytes>::Decoder
        });

        let field_type = &field.ty;
        field_types.push(quote!{
            #field_type
        });

        i += 1;
        
    }

    let struct_ident = Ident::new(format!("{}Decoder", name.to_string()).trim(), Span::call_site());
    let last_count = quote!{ #i };

    let gen = quote! {
        #[derive(Clone)]
        pub struct #struct_ident #impl_generics {
            counter:u8,
            #(#decode_field_idents:#decode_field_types),* ,
            #(#attrs_names:Option<#field_types>),*
        }

        impl #impl_generics to_from_bytes::ByteDecoder<#name #ty_generics> for #struct_ident #ty_generics {
            fn decode_byte(&mut self, bytes:&mut Vec<u8>, byte:u8) -> Option<#name #ty_generics> {
                match self.counter {
                    #(#counter_steps => {
                        match <#decode_field_types as to_from_bytes::ByteDecoder<#field_types>>::decode_byte(&mut self.#decode_field_idents, bytes, byte) {
                            Some(val) => {
                                self.#attrs_names = Some(val);
                                self.counter += 1;
                                bytes.clear();
                            },
                            None => ()
                        };
                    }),*
                    
                    _ => ()
                }
                if self.counter == #last_count {
                    bytes.clear();
                    Some(#name (
                        #(self.#attrs_names.as_ref().unwrap().clone()),*
                    ))
                }
                else {
                    None
                }
                
            }
            fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(#name #ty_generics, usize)> {
                let mut i = 0;
                while self.counter < #last_count && i < slice_to_decode.len() {
                    match self.counter {
                        #(#counter_steps => {
                            match <#decode_field_types as to_from_bytes::ByteDecoder<#field_types>>::decode_slice_borrow(&mut self.#decode_field_idents, bytes, &slice_to_decode[i..]) {
                                Some((val,bytes_read)) => {
                                    self.#attrs_names = Some(val);
                                    self.counter += 1;
                                    i += bytes_read;
                                    bytes.clear();
                                },
                                None => i = slice_to_decode.len()
                            };
                        }),*
                        
                        _ => ()
                    }
                    
                }
                if self.counter == #last_count {
                    bytes.clear();
                    Some((#name(
                        #(#attrs_names:self.#attrs_names.as_ref().unwrap().clone()),*
                    ), i))
                }
                else {
                    None
                }
            }
        }
        

        impl #impl_generics to_from_bytes::FromBytes for #name #ty_generics {
            type Decoder = #struct_ident #ty_generics;
            fn get_decoder() -> Self::Decoder {
                #struct_ident {
                    counter:0,
                    #(#decode_field_idents:<#field_types as to_from_bytes::FromBytes>::get_decoder()),* ,
                    #(#attrs_names:None),*
                }
            }
        }
    };
    gen.into()
}

fn impl_decodebytes_struct(ast:&syn::DeriveInput, datastruct:&DataStruct) -> TokenStream {
    match &datastruct.fields {
        Named(fields_named) => decodebytes_named_struct(ast, fields_named),
        Unnamed(fields_unnamed) => decodebytes_unnamed_struct(ast, fields_unnamed),
        _ => panic!("ffdfs")  
    }
}

fn impl_decodebytes_enum(ast:&syn::DeriveInput, dataenum:&DataEnum) -> TokenStream {
    
    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    
    let decoder_ident = Ident::new(format!("{}Decoder", name.to_string()).trim(), Span::call_site());

    let decoder_enum_ident = Ident::new(format!("{}DecoderEnum", name.to_string()).trim(), Span::call_site());

    let mut unit_decodes = Vec::new();

    let mut decoder_enum_variants = Vec::new();

    let mut decoders_from_numbers = Vec::new();

    let mut byte_decode_variants = Vec::new();

    let mut slice_decode_variants = Vec::new();

    let mut variant_number = 0_usize;
    for variant in &dataenum.variants {
        let ident = &variant.ident;
        match &variant.fields {
            Named(fields_named) => {
                let (enum_variant, from_number, byte_decode, slice_decode) = get_enum_tokens_named(ast, fields_named, ident, &ast.ident, &decoder_enum_ident,variant_number as u8);
                decoder_enum_variants.push(enum_variant.into());
                decoders_from_numbers.push(from_number.into());
                byte_decode_variants.push(byte_decode.into());
                slice_decode_variants.push(slice_decode.into());
            },
            Unnamed(fields_unnamed) => {
                let (enum_variant, from_number, byte_decode, slice_decode) = get_enum_tokens_unnamed(ast, fields_unnamed, ident, &ast.ident, &decoder_enum_ident,variant_number as u8);
                decoder_enum_variants.push(enum_variant.into());
                decoders_from_numbers.push(from_number.into());
                byte_decode_variants.push(byte_decode.into());
                slice_decode_variants.push(slice_decode.into());

            },
            Unit => {
                let variant_u8 = variant_number as u8;
                let enum_variant = Ident::new(format!("{}Decode", ident.to_string()).trim(), Span::call_site());
                
                decoder_enum_variants.push(quote!{
                    #enum_variant
                });
                decoders_from_numbers.push(quote!{
                    #variant_u8 => (Self::#enum_variant, true)
                });
                byte_decode_variants.push(quote!{
                    #decoder_enum_ident::#enum_variant => None
                });
                slice_decode_variants.push(quote!{
                    #decoder_enum_ident::#enum_variant => None
                });
                unit_decodes.push(quote!{
                    #decoder_enum_ident::#enum_variant => #name::#ident
                });

            }
        }
        variant_number += 1;
    }

    let units_decoded = match unit_decodes.len() {
        0 => quote! {},
        _ => quote! {#(#unit_decodes),*,}
    };
    
    let gen = quote! {
        #[derive(Clone)]
        enum #decoder_enum_ident #impl_generics {
            MissingVariant,
            #(#decoder_enum_variants),*
        }
        #[derive(Clone)]
        pub struct #decoder_ident #impl_generics {
            counter:u8,
            decoder_enum:#decoder_enum_ident #ty_generics
        }

        impl #impl_generics #decoder_enum_ident #ty_generics {
            fn from_variant_number(number:u8) -> (Self, bool) {
                match number {
                    #(#decoders_from_numbers),*,
                    _ => (Self::MissingVariant, false)
                }
            }
            fn unit_to_decoded(&self) -> #name #ty_generics {
                match &self {
                    #units_decoded
                    _ => panic!("gagagag"),
                }
            }
        }

        impl #impl_generics to_from_bytes::ByteDecoder<#name #ty_generics> for #decoder_ident #ty_generics {
            fn decode_byte(&mut self, bytes:&mut Vec<u8>, byte:u8) -> Option<#name #ty_generics> {
                match &mut self.decoder_enum {
                    #(#byte_decode_variants),*,
                    MissingVariant => {
                        let (enum_decoder, is_unit)= #decoder_enum_ident::from_variant_number(byte);
                        if is_unit {
                            Some(enum_decoder.unit_to_decoded())
                        }
                        else {
                            self.decoder_enum = enum_decoder;
                            None
                        }
                    },
                }
            }
            fn decode_slice_borrow(&mut self, bytes:&mut Vec<u8>, slice_to_decode:&[u8]) -> Option<(#name #ty_generics, usize)> {
                match &mut self.decoder_enum {
                    #(#slice_decode_variants),*,
                    MissingVariant => {
                        let byte = slice_to_decode[0];
                        let (enum_decoder, is_unit)= #decoder_enum_ident::from_variant_number(byte);
                        if is_unit {
                            Some((enum_decoder.unit_to_decoded(), 1))
                        }
                        else {
                            self.decoder_enum = enum_decoder;
                            self.decode_slice_borrow(bytes, &slice_to_decode[1..])
                        }
                    },
                }
            }
            
        }

        impl #impl_generics to_from_bytes::FromBytes for #name #ty_generics {
            type Decoder = #decoder_ident #ty_generics;
            fn get_decoder() -> Self::Decoder {
                #decoder_ident {
                    counter:0,
                    decoder_enum:#decoder_enum_ident::MissingVariant
                }
            }
        }
    };
    gen.into()
}

fn get_enum_tokens_named(ast:&syn::DeriveInput, fields_named:&FieldsNamed, ident:&Ident,enum_ident:&Ident, decode_enum_ident:&Ident,variant_number:u8) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let variant_name = Ident::new(format!("{}Decoder", ident.to_string()).trim(), Span::call_site());
    let mut field_names = Vec::new();
    let mut field_declarations = Vec::new();
    let mut counter_steps = Vec::new();
    
    let mut original_field_names = Vec::new();
    let mut decoder_declarations = Vec::new();
    let mut slice_counter_steps = Vec::new();

    let mut counter = 0_u8;
    for field in fields_named.named.iter() {
        let field_ident = field.ident.as_ref().unwrap();
        let decode_field_name = Ident::new(format!("{}_decoder", field_ident.to_string()).trim(), Span::call_site());
        original_field_names.push(quote!{#field_ident});
        field_names.push(quote!{#field_ident});
        field_names.push(quote!{#decode_field_name});


        let field_type = &field.ty;

        field_declarations.push(quote!{#field_ident:Option<#field_type>});
        field_declarations.push(quote!{#decode_field_name:<#field_type as to_from_bytes::FromBytes>::Decoder});

        decoder_declarations.push(quote!{#field_ident:None});
        decoder_declarations.push(quote!{#decode_field_name:<#field_type as to_from_bytes::FromBytes>::get_decoder()});

        counter_steps.push(quote!{#counter => match #decode_field_name.decode_byte(bytes,byte) {
            Some(val) => {
                *#field_ident = Some(val);
                self.counter += 1;
                bytes.clear();
            },
            None => ()
        }});
        slice_counter_steps.push(quote! {
            #counter => {
                match #decode_field_name.decode_slice_borrow(bytes, &slice_to_decode[i..]) {
                    Some((val,bytes_read)) => {
                        *#field_ident = Some(val);
                        self.counter += 1;
                        i += bytes_read;
                        bytes.clear();
                    },
                    None => i = slice_to_decode.len()
                };
            }
        });
        counter += 1;
    }
    let number_of_fields = fields_named.named.len() as u8;
    let mut final_counter = quote!{
    if self.counter == #number_of_fields {
        Some(#enum_ident::#ident{#(#original_field_names:#original_field_names.as_ref().unwrap().clone()),*})
    }
    else {
        None
    }};
    (
        quote!{#variant_name{#(#field_declarations),*}}.into(),
        quote!{#variant_number => (Self::#variant_name{#(#decoder_declarations),*}, false)}.into(),
        quote!{#decode_enum_ident::#variant_name{#(#field_names),*} => {match self.counter {
            #(#counter_steps),* ,
            _ => ()
        }
        #final_counter
    }}.into(),
    quote! {
        #decode_enum_ident::#variant_name{#(#field_names),*} => {
            let mut i = 0;
            while self.counter < #number_of_fields && i < slice_to_decode.len() {
                match self.counter {
                    #(#slice_counter_steps),*,
                    
                    _ => ()
                }
                
            }
            if self.counter == #number_of_fields {
                bytes.clear();
                Some((#enum_ident::#ident{#(#original_field_names:#original_field_names.as_ref().unwrap().clone()),*}, i))
            }
            else {
                None
            }
        }
    }.into()
)
}   

fn get_enum_tokens_unnamed(ast:&syn::DeriveInput, fields_named:&FieldsUnnamed, ident:&Ident,enum_ident:&Ident,decode_enum_ident:&Ident ,variant_number:u8) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let variant_name = Ident::new(format!("{}Decoder", ident.to_string()).trim(), Span::call_site());
    let mut field_names = Vec::new();
    let mut field_declarations = Vec::new();
    let mut counter_steps = Vec::new();
    let mut slice_counter_steps = Vec::new();
    
    let mut original_field_names = Vec::new();
    let mut decoder_declarations = Vec::new();

    let mut counter = 0_u8;
    for field in fields_named.unnamed.iter() {
        let field_ident = Ident::new(format!("ident_{}", counter).trim(), Span::call_site());
        let decode_field_name = Ident::new(format!("{}_decoder", field_ident.to_string()).trim(), Span::call_site());
        original_field_names.push(quote!{#field_ident});
        field_names.push(quote!{#field_ident});
        field_names.push(quote!{#decode_field_name});


        let field_type = &field.ty;

        field_declarations.push(quote!{#field_ident:Option<#field_type>});
        field_declarations.push(quote!{#decode_field_name:<#field_type as to_from_bytes::FromBytes>::Decoder});

        decoder_declarations.push(quote!{#field_ident:None});
        decoder_declarations.push(quote!{#decode_field_name:<#field_type as to_from_bytes::FromBytes>::get_decoder()});

        counter_steps.push(quote!{#counter => match #decode_field_name.decode_byte(bytes,byte) {
            Some(val) => {
                *#field_ident = Some(val);
                self.counter += 1;
                bytes.clear();
            },
            None => ()
        }});
        slice_counter_steps.push(quote! {
            #counter => {
                match #decode_field_name.decode_slice_borrow(bytes, &slice_to_decode[i..]) {
                    Some((val,bytes_read)) => {
                        *#field_ident = Some(val);
                        self.counter += 1;
                        i += bytes_read;
                        bytes.clear();
                    },
                    None => i = slice_to_decode.len()
                };
            }
        });
        counter += 1;
    }
    let number_of_fields = fields_named.unnamed.len() as u8;
    let mut final_counter = quote!{
    if self.counter == #number_of_fields {
        Some(#enum_ident::#ident(#(#original_field_names.as_ref().unwrap().clone()),*))
    }
    else {
        None
    }};
    (
        quote!{#variant_name{#(#field_declarations),*}}.into(),
        quote!{#variant_number => (Self::#variant_name{#(#decoder_declarations),*},false)}.into(),
        quote!{#decode_enum_ident::#variant_name{#(#field_names),*} => {match self.counter {
        #(#counter_steps),* ,
            _ => ()
        }
        #final_counter
    }}.into(),

    quote! {
        #decode_enum_ident::#variant_name{#(#field_names),*} => {
            let mut i = 0;
            while self.counter < #number_of_fields && i < slice_to_decode.len() {
                match self.counter {
                    #(#slice_counter_steps),*,
                    
                    _ => ()
                }
                
            }
            if self.counter == #number_of_fields {
                bytes.clear();
                Some((#enum_ident::#ident(#(#original_field_names.as_ref().unwrap().clone()),*), i))
            }
            else {
                None
            }
        }
        
    }.into()
)
}

fn impl_decodebytes(ast: &syn::DeriveInput) -> TokenStream {
    match &ast.data {
      Data::Struct(datastruct) => impl_decodebytes_struct(ast, datastruct),
      Data::Enum(dataenum) => impl_decodebytes_enum(ast, dataenum),
      _ => panic!("eeaeae")
    }
}