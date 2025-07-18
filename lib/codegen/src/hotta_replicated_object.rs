use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    DataStruct, Ident, LitStr,
    parse::{Parse, ParseStream},
};

struct PropertyAttr {
    name: LitStr,
}

pub fn impl_trait_for_struct(ident: &Ident, data: &DataStruct) -> TokenStream {
    const PROPERTY_ATTR_NAME: &str = "property";

    let mut property_replication = TokenStream::new();
    let property_count = data.fields.len();

    for field in data.fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident(PROPERTY_ATTR_NAME))
            .unwrap_or_else(|| {
                panic!("field {field_name} is missing #[{PROPERTY_ATTR_NAME}] attribute")
            });

        let PropertyAttr { name } = attr.parse_args::<PropertyAttr>().unwrap();

        property_replication.extend(quote! {
            properties.push(crate::logic::hotta::HottaReplicatedObjectProperty {
                name: ::fadia_engine::util::FName::Custom(String::from(#name)),
                datas: crate::logic::hotta::HottaReplicatedProperty::replicate(&self.#field_name),
            });
        });
    }

    quote! {
        impl crate::logic::hotta::HottaReplicatedObject for #ident {
            fn replicate(&self, owner: ::fadia_engine::FNetworkGUID) -> crate::logic::hotta::HottaReplicatedObjectPropertyContainer {
                let mut properties = ::std::vec::Vec::with_capacity(#property_count);

                #property_replication

                crate::logic::hotta::HottaReplicatedObjectPropertyContainer {
                    owner,
                    properties
                }
            }
        }
    }
}

impl Parse for PropertyAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
        })
    }
}
