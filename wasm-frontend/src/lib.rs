use std::{cell::UnsafeCell, collections::HashMap, path::Path};

use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use std::sync::*;

struct Endpoint {
    fn_name: Ident,
    endpoint: String,
    file: TokenStream,
    content_type: &'static str,
}

const OUT_DIR: &'static str = env!["OUT_DIR"];

#[proc_macro]
pub fn service_fontend_endpoints(_: TS) -> TS {
    let endpoints = include_dir("/".into(), format!("{OUT_DIR}/webywasm/dist"));
    let fn_names = endpoints.iter().map(|Endpoint { fn_name, .. }| fn_name);

    quote! {( #(#fn_names),* )}.into()
}

#[proc_macro]
pub fn create_fontend_endpoints(_: TS) -> TS {
    let endpoints = include_dir("/".into(), format!("{OUT_DIR}/webywasm/dist"));
    let x = endpoints
        .iter()
        .map(
            |Endpoint {
                 endpoint,
                 file,
                 fn_name,
                 content_type,
             }| {
                quote! {
                    #[::actix_web::get(#endpoint)]
                    async fn #fn_name() -> impl ::actix_web::Responder  {
                        ::actix_web::HttpResponse::Ok().content_type(#content_type).body(#file.as_slice())
                    }
                }
            },
        )
        .collect::<TokenStream>();

    dbg!(&x.to_string());

    x.into()
}

fn include_dir(path: String, src: impl AsRef<Path>) -> Vec<Endpoint> {
    let mut out = vec![];
    let dir = std::fs::read_dir(&src).unwrap();
    for entry in dir {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        let file_path = entry_path.to_str().unwrap();
        let filetype = entry.file_type().unwrap();
        let file_name = entry.file_name();
        let name = file_name.to_str().unwrap();
        let extention = name.split(".").last().unwrap();
        if filetype.is_dir() {
            out.append(&mut include_dir(format!("{path}{name}/"), entry.path()));
        } else {
            out.push(Endpoint {
                fn_name: Ident::new(
                    format!("{path}{name}")
                        .replace(['/', '-', '.'], "_")
                        .as_str(),
                    Span::call_site(),
                ),
                endpoint: format!("{path}{name}"),
                content_type: content_type_based_on_extention(extention),
                file: quote! { include_bytes!(#file_path) },
            })
        }
    }
    return out;
}

fn content_type_based_on_extention(extention: &str) -> &'static str {
    let map = [
        ("js", "text/javascript"),
        ("html", "text/html"),
        ("css", "text/css"),
        ("css", "text/css"),
        ("wasm", "application/wasm"),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();

    map.get(extention)
        .expect(format!("failed to find filetype {extention} in list, pls add").as_str())
}
