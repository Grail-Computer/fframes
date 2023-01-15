extern crate proc_macro2;

mod node;
mod nodes_to_format;
mod parser;
mod validate_svg;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

use syn::{
    parse::{ParseStream, Parser as _},
    Result,
};

use node::Node;
use parser::{Parser, ParserOptions};

use crate::nodes_to_format::prepare_svg_nodes_for_format_statement;

mod punctuation {
    use syn::custom_punctuation;

    custom_punctuation!(Dash, -);
}

fn parse(tokens: proc_macro::TokenStream) -> Result<Vec<Node>> {
    let parser = move |input: ParseStream| Parser::new(ParserOptions::default()).parse(input);

    parser.parse(tokens)
}

#[proc_macro]
pub fn svgr(tokens: TokenStream) -> TokenStream {
    let fframes_crate_ident = match proc_macro_crate::crate_name("fframes")
        .expect("fframes crate must be present in Cargo.toml")
    {
        proc_macro_crate::FoundCrate::Itself => Ident::new("crate", Span::call_site()),
        proc_macro_crate::FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
    };

    match parse(tokens) {
        Ok(nodes) => {
            let (html_string, values, animations) =
                prepare_svg_nodes_for_format_statement(nodes, &fframes_crate_ident);

            quote! {
            {
                lazy_static::lazy_static! {
                    #(#animations)*
                }

                #[allow(unused_braces)]
                #fframes_crate_ident::Svgr {
                    value: format!(#html_string, #(#values),*)
                }
            }
            }
        }
        Err(error) => error.to_compile_error(),
    }
    .into()
}
