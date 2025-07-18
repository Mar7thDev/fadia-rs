use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Fields, Ident};

pub fn impl_replicated_property_for_newtype(ident: &Ident, data: &DataStruct) -> TokenStream {
    let Fields::Unnamed(fields) = &data.fields else {
        panic!(
            "ReplicatedProperty newtype should wrap an existing ReplicatedProperty type in a tuple struct"
        );
    };

    assert!(
        fields.unnamed.len() == 1,
        "ReplicatedProperty newtype shouldn't have any extra fields"
    );

    quote! {
        impl ::fadia_engine::replication::property::ReplicatedProperty for #ident {
            fn is_changed(&self) -> bool {
                self.0.is_changed()
            }

            fn acknowledge_changes(&mut self) {
                self.0.acknowledge_changes()
            }

            fn serialize(&self, w: &mut ::fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
                self.0.serialize(w)
            }
        }
    }
}
