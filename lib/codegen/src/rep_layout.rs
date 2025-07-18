use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, DataStruct, Ident, LitInt, Token,
    parse::{Parse, ParseStream},
};

#[derive(Default)]
struct RepAttribute {
    handle: Option<LitInt>,
    index: Option<LitInt>,
    ignore: bool,
}

struct MaxRepIndexAttribute {
    max_rep_index: LitInt,
}

pub fn impl_rep_layout(ident: &Ident, data: &DataStruct, attrs: &[Attribute]) -> TokenStream {
    const REP_ATTR_NAME: &str = "rep";
    const MAX_REP_INDEX_ATTR_NAME: &str = "max_rep_index";
    const MAX_REP_INDEX_DEFAULT_LIT: &str = "1";

    let mut fields_with_handle: Vec<&Ident> = Vec::new();
    let mut fields_with_rep_index: Vec<&Ident> = Vec::new();
    let mut layout_serialization = TokenStream::new();
    let mut custom_properties_serialization = TokenStream::new();

    let max_rep_index = attrs
        .iter()
        .find(|attr| attr.path().is_ident(MAX_REP_INDEX_ATTR_NAME))
        .map(|attr| {
            attr.parse_args::<MaxRepIndexAttribute>()
                .unwrap_or_else(|_| {
                    panic!("invalid args specified for #[{MAX_REP_INDEX_ATTR_NAME}]")
                })
        });

    let max_rep_index = max_rep_index
        .map(|attr| attr.max_rep_index)
        .unwrap_or(LitInt::new(MAX_REP_INDEX_DEFAULT_LIT, ident.span()));

    for field in data.fields.iter() {
        let name = field.ident.as_ref().unwrap();
        let attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident(REP_ATTR_NAME))
            .unwrap_or_else(|| panic!("field {name} is missing #[{REP_ATTR_NAME}] attribute"));

        let attr = attr.parse_args::<RepAttribute>().unwrap();

        if attr.ignore {
            continue;
        }

        if let Some(handle) = attr.handle.as_ref() {
            fields_with_handle.push(name);

            layout_serialization.extend(quote! {
                if full || ::fadia_engine::replication::property::ReplicatedProperty::is_changed(&self.#name) {
                    ::fadia_engine::util::PackedBitWriteExt::write_packed_int(w, #handle)?;
                    ::fadia_engine::replication::property::ReplicatedProperty::serialize(&self.#name, w)?;
                }

            });
        }

        if let Some(rep_index) = attr.index.as_ref() {
            fields_with_rep_index.push(name);

            custom_properties_serialization.extend(quote! {
                if full || ::fadia_engine::replication::property::ReplicatedProperty::is_changed(&self.#name) {
                    let mut data = Vec::new();
                    let mut out = ::fadia_engine::util::OutBitWriter::new(&mut data);

                    ::fadia_engine::replication::property::ReplicatedProperty::serialize(&self.#name, &mut out)?;
                    ::bitstream_io::BitWrite::write_bit(&mut out, true)?; // termination bit
                    ::bitstream_io::BitWrite::byte_align(&mut out)?;

                    output.push((#rep_index, data.into_boxed_slice()));
                }
            });
        }
    }

    let is_layout_empty = fields_with_handle.is_empty();

    quote! {
        impl ::fadia_engine::replication::RepLayout for #ident {
            fn rep_layout_changed(&self) -> bool {
                false #(|| ::fadia_engine::replication::property::ReplicatedProperty::is_changed(&self.#fields_with_handle))*
            }

            fn custom_properties_changed(&self) -> bool {
                false #(|| ::fadia_engine::replication::property::ReplicatedProperty::is_changed(&self.#fields_with_rep_index))*
            }

            fn acknowledge_changes(&mut self) {
                #(::fadia_engine::replication::property::ReplicatedProperty::acknowledge_changes(&mut self.#fields_with_handle);)*
                #(::fadia_engine::replication::property::ReplicatedProperty::acknowledge_changes(&mut self.#fields_with_rep_index);)*
            }

            fn serialize_layout_properties(&self, w: &mut ::fadia_engine::util::OutBitWriter, full: bool) -> ::std::io::Result<()> {
                #layout_serialization

                // Null Handle for termination
                ::fadia_engine::util::PackedBitWriteExt::write_packed_int(w, 0)
            }

            fn serialize_custom_properties(&self, full: bool) -> ::std::io::Result<Vec<(u32, Box<[u8]>)>> {
                let mut output = Vec::new();
                #custom_properties_serialization

                Ok(output)
            }

            fn is_empty(&self) -> bool {
                #is_layout_empty
            }

            fn max_rep_index(&self) -> u32 {
                #max_rep_index
            }
        }
    }
}

impl Parse for RepAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attr = RepAttribute::default();

        loop {
            let ident = input.parse::<Ident>()?;

            if ident == "ignore" {
                if attr.handle.is_some() || attr.index.is_some() {
                    panic!("invalid #[rep] attribute specified");
                } else {
                    attr.ignore = true;
                    return Ok(attr);
                }
            }

            let _ = input.parse::<Token![=]>()?;
            let literal = input.parse::<LitInt>()?;

            if ident == "handle" {
                assert!(
                    attr.handle.is_none(),
                    "'handle' is specified multiple times"
                );
                attr.handle = Some(literal);
            } else if ident == "index" {
                assert!(attr.index.is_none(), "'index' is specified multiple times");
                attr.index = Some(literal);
            } else {
                panic!("unexpected attribute value: {ident}")
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(attr)
    }
}

impl Parse for MaxRepIndexAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(MaxRepIndexAttribute {
            max_rep_index: {
                let lit = input.parse()?;
                assert!(input.is_empty(), "expected: #[max_rep_index(value)]");
                lit
            },
        })
    }
}
