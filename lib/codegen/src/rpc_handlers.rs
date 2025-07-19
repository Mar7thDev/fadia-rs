use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, ImplItem, LitInt, PatType, Token, Type, parse::Parse};

#[derive(PartialEq)]
enum RpcDirection {
    Client,
    Server,
}

struct RpcAttr {
    rep_index: LitInt,
    _comma: Token![,],
    direction: RpcDirection,
}

pub fn impl_rpc_handler_trait(impl_items: &[ImplItem], self_ty: &Type) -> TokenStream {
    const RPC_ATTR_NAME: &str = "rpc";

    let mut out_fns = Vec::new();
    let mut fn_wrappers = Vec::new();
    let mut handler_match_arms = Vec::new();

    for item in impl_items.iter() {
        let ImplItem::Fn(item_fn) = item else {
            panic!("all items in impl block should be functions");
        };

        let RpcAttr {
            rep_index,
            direction,
            ..
        } = item_fn
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident(RPC_ATTR_NAME))
            .map(|attr| {
                attr.parse_args::<RpcAttr>().unwrap_or_else(|err| {
                    panic!("failed to parse #[{RPC_ATTR_NAME}] attribute: {err}")
                })
            })
            .unwrap_or_else(|| panic!("every function should have #[{RPC_ATTR_NAME}] attribute"));

        let fn_name = &item_fn.sig.ident;

        if direction == RpcDirection::Server {
            let wrapper_name = format_ident!("{fn_name}_wrapped");

            handler_match_arms.push(quote! {
                #rep_index => Some(Self::#wrapper_name),
            });

            let mut rep_class_serialized_flag = TokenStream::new();
            let mut arg_deserialization = Vec::new();

            if item_fn.sig.inputs.len() > 1 {
                rep_class_serialized_flag.extend(quote! {
                    ::bitstream_io::BitRead::read_bit(&mut r)?;
                });

                for _ in 1..item_fn.sig.inputs.len() {
                    arg_deserialization.push(quote! {
                        crate::logic::rpc::RpcArgument::deserialize(&mut r)?
                    });
                }
            }

            fn_wrappers.push(quote! {
                fn #wrapper_name(context: crate::logic::rpc::RpcContext, rpc: crate::logic::replication::InRPC) -> ::std::io::Result<()> {
                    let mut r = ::fadia_engine::util::InBitReader::new(::std::io::Cursor::new(rpc.data.as_ref()));
                    #rep_class_serialized_flag
                    Self::#fn_name(context, #(#arg_deserialization),*);
                    Ok(())
                }
            });

            let sig = &item_fn.sig;
            let block = &item_fn.block;

            out_fns.push(quote! {
                #sig #block
            });
        } else {
            let vis = &item_fn.vis;
            let sig = &item_fn.sig;

            let mut arg_serialization = TokenStream::new();

            // If RPC doesn't have arguments at all, bool flag isn't encoded
            if item_fn.sig.inputs.len() > 1 {
                arg_serialization.extend(quote! {
                    out.write_bit(true).unwrap();
                });

                for arg in item_fn.sig.inputs.iter().skip(1) {
                    let FnArg::Typed(PatType { pat, .. }) = arg else {
                        panic!("invalid argument encountered");
                    };

                    arg_serialization.extend(quote! {
                        crate::logic::rpc::RpcArgument::serialize(&#pat, &mut out).unwrap();
                    });
                }
            }

            out_fns.push(quote! {
                #vis #sig -> (u32, Box<[u8]>) {
                    use ::bitstream_io::BitWrite;

                    let mut buffer = Vec::new();
                    let mut out = ::fadia_engine::util::OutBitWriter::new(&mut buffer);

                    #arg_serialization

                    out.write_bit(true).unwrap(); // termination bit
                    out.byte_align().unwrap();

                    (#rep_index, buffer.into_boxed_slice())
                }
            });
        }
    }

    quote! {
        impl #self_ty {
            #(#out_fns)*
            #(#fn_wrappers)*
        }

        impl crate::logic::rpc::RpcHandler for #self_ty {
            fn get_handler_func(&self, rep_index: u32) -> Option<crate::logic::rpc::RpcHandlerFunc> {
                match rep_index {
                    #(#handler_match_arms)*
                    _ => None,
                }
            }
        }
    }
}

impl Parse for RpcAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        const RPC_DIRECTION_CLIENT: &str = "client";
        const RPC_DIRECTION_SERVER: &str = "server";

        Ok(Self {
            rep_index: input.parse()?,
            _comma: input.parse()?,
            direction: match input.parse::<Ident>()? {
                ident if ident == RPC_DIRECTION_CLIENT => RpcDirection::Client,
                ident if ident == RPC_DIRECTION_SERVER => RpcDirection::Server,
                invalid => panic!("invalid rpc direction specified: {invalid}"),
            },
        })
    }
}
