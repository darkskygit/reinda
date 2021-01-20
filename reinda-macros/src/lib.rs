use std::convert::TryInto;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;

mod parse;


#[proc_macro]
pub fn assets(input: TokenStream1) -> TokenStream1 {
    run(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}


fn run(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse2::<Input>(input)?;

    let mut match_arms = Vec::new();
    let mut asset_defs = Vec::new();

    for (path, asset) in input.assets {
        let idx: u32 = match_arms.len().try_into().expect("you have more than 2^32 assets?!");
        match_arms.push(quote! {
            #path => Some(reinda::AssetId(#idx)),
        });

        let hash = asset.hash;
        let serve = asset.serve;
        let template = asset.template;
        let dynamic = asset.dynamic;
        let append = match asset.append {
            Some(s) => quote! { Some(#s) },
            None => quote! { None },
        };
        let prepend = match asset.prepend {
            Some(s) => quote! { Some(#s) },
            None => quote! { None },
        };
        let content_field = if cfg!(debug_assertions) {
            quote! {}
        } else {
            let full_path = resolve_path(&input.base_path, &path);
            quote! { content: include_bytes!(#full_path) }
        };

        asset_defs.push(quote! {
            reinda::AssetDef {
                path: #path,
                serve: #serve,
                hash: #hash,
                dynamic: #dynamic,
                template: #template,
                append: #append,
                prepend: #prepend,
                #content_field
            }
        });
    }

    let base_path = &input.base_path;
    Ok(quote! {
        reinda::Setup {
            base_path: #base_path,
            assets: &[#( #asset_defs ,)*],
            path_to_id: reinda::PathToIdMap(|s: &str| -> Option<reinda::AssetId> {
                match s {
                    #( #match_arms )*
                    _ => None,
                }
            }),
        }
    })
}

#[derive(Debug)]
struct Input {
    base_path: Option<String>,
    assets: Vec<(String, Asset)>,
}

#[derive(Debug)]
struct Asset {
    hash: bool,
    serve: bool,
    dynamic: bool,
    template: bool,
    append: Option<String>,
    prepend: Option<String>,
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            hash: false,
            serve: true,
            dynamic: false,
            template: false,
            append: None,
            prepend: None,
        }
    }
}

fn resolve_path(base: &Option<String>, path: &str) -> String {
    match base {
        Some(base) => {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR not set");
            format!("{}/{}/{}", manifest, base, &path)
        },
        None => path.to_string(),
    }
}
