mod impls;
mod parse_attributes;
mod parse_enum;
mod templates;

extern crate proc_macro;

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::DeriveInput;
use tera::{Context, Tera};

use crate::parse_attributes::{parse_custom_error_config, ErrorConfig};

const ENUM_CONV_LIFETIME: &str = "'enum_conv";

use crate::parse_enum::{
    create_marker_enums, fetch_fields_from_enum, fetch_impl_generics,
    fetch_name_with_generic_params, get_marker,
};

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn EnumConversions(args: TokenStream, input: TokenStream) -> TokenStream {
    let error_config = parse_custom_error_config(args.into());
    let enum_ast = syn::parse(input).unwrap();

    impl_conversions(error_config, enum_ast)
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn DeriveTryFrom(_: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Implements ContainsVariant, GetVariant, SetVariant, and CreateVariantFrom traits
fn impl_conversions(error_config: ErrorConfig, mut ast: DeriveInput) -> TokenStream {
    let tera = templates::templater();
    let name = &ast.ident.to_string();
    let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
    let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);

    let field_map = fetch_fields_from_enum(&mut ast);
    let mut tokens: TokenStream = ast.to_token_stream().to_string().parse().unwrap();

    tokens.extend::<TokenStream>(create_marker_enums(name, &field_map).parse().unwrap());
    tokens.extend::<TokenStream>(
        impls::impl_get_variant(name, &fullname, &impl_generics, &field_map, &tera)
            .parse()
            .unwrap(),
    );
    tokens.extend::<TokenStream>(
        impls::impl_try_from(
            name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        )
        .parse()
        .unwrap(),
    );
    tokens.extend::<TokenStream>(
        impls::impl_try_to(
            name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        )
        .parse()
        .unwrap(),
    );
    tokens.extend::<TokenStream>(
        impls::impl_from(&fullname, &impl_generics, &field_map, &tera)
            .parse()
            .unwrap(),
    );
    tokens
}
