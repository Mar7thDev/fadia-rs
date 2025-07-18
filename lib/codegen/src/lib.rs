use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Data, DeriveInput, ItemEnum, ItemImpl, ItemStruct, parse_macro_input};

mod hotta_replicated_object;
mod newtype_util;
mod rep_layout;
mod rpc_handlers;

#[proc_macro_derive(RepLayout, attributes(rep, max_rep_index))]
pub fn derive_rep_layout(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let Data::Struct(data) = input.data else {
        panic!("RepLayout can be represented only with a struct");
    };

    rep_layout::impl_rep_layout(&input.ident, &data, &input.attrs).into()
}

#[proc_macro_derive(ReplicatedProperty)]
pub fn derive_replicated_property(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let Data::Struct(data) = input.data else {
        panic!("ReplicatedProperty newtype can be represented only with a struct");
    };

    newtype_util::impl_replicated_property_for_newtype(&input.ident, &data).into()
}

#[proc_macro_derive(HottaReplicatedObject, attributes(property))]
pub fn derive_hotta_replicated_object(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let Data::Struct(data) = input.data else {
        panic!("HottaReplicatedObject can be represented only with a struct");
    };

    hotta_replicated_object::impl_trait_for_struct(&input.ident, &data).into()
}

#[proc_macro_attribute]
pub fn rpc_handlers(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // generates RpcHandler implementation based on an 'impl' block

    let input = parse_macro_input!(item as ItemImpl);
    assert!(
        input.trait_.is_none(),
        "#[rpc_handler] should be applied on a type-specific impl block"
    );

    rpc_handlers::impl_rpc_handler_trait(&input.items, &input.self_ty).into()
}

#[proc_macro_attribute]
pub fn dummy_rpc_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // provides dummy implementation of RpcHandler trait

    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;

    quote! {
        #input

        impl crate::logic::rpc::RpcHandler for #ident {
            fn get_handler_func(&self, _rep_index: u32) -> Option<crate::logic::rpc::RpcHandlerFunc> {
                None
            }
        }

    }.into()
}

#[proc_macro_attribute]
pub fn replicated_enum(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // generates a ReplicatedProperty wrapper struct for enum type

    let item = parse_macro_input!(item as ItemEnum);
    let ident = &item.ident;

    let rep_ident = format_ident!("Property{ident}");
    let max_discrim = item
        .variants
        .iter()
        .map(|var| {
            var.discriminant
                .as_ref()
                .expect("every enum variant should have a discriminant")
                .1
                .to_token_stream()
                .to_string()
                .parse::<u64>()
                .expect("failed to parse discriminant")
        })
        .max()
        .expect("enum should have at least one enum variant");

    let bits_needed = 64 - max_discrim.leading_zeros();
    let mut serialization = proc_macro2::TokenStream::new();

    for var in item.variants.iter() {
        let (_, discrim) = &var.discriminant.as_ref().unwrap();
        let var_ident = &var.ident;
        serialization.extend(quote! {
            #ident::#var_ident => ::bitstream_io::BitWrite::write(w, #bits_needed, #discrim),
        });
    }

    quote! {
        #item

        #[derive(Debug)]
        pub struct #rep_ident {
            value: #ident,
            changed: bool,
        }

        impl #rep_ident {
            pub fn new(value: #ident) -> Self {
                Self {
                    value,
                    changed: false,
                }
            }

            pub fn set_value(&mut self, new_value: #ident) {
                if self.value != new_value {
                    self.value = new_value;
                    self.changed = true;
                }
            }

            pub fn get(&self) -> #ident {
                self.value
            }
        }

        impl ::fadia_engine::replication::property::ReplicatedProperty for #rep_ident {
            fn is_changed(&self) -> bool {
                self.changed
            }

            fn acknowledge_changes(&mut self) {
                self.changed = false;
            }

            fn serialize(&self, w: &mut ::fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
                match self.value {
                    #serialization
                }
            }
        }
    }
    .into()
}
