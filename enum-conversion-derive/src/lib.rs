mod impls;
mod parse_attributes;
mod parse_enum;
mod templates;

extern crate proc_macro;

use proc_macro::TokenStream;
use std::collections::HashMap;
use syn::DeriveInput;
use tera::{Context, Tera};

const ENUM_CONV_LIFETIME: &str = "'enum_conv";

use crate::parse_enum::{
    create_marker_enums, fetch_fields_from_enum, fetch_impl_generics,
    fetch_name_with_generic_params, get_marker,
};

#[proc_macro_derive(EnumConversions, attributes(DeriveTryFrom, TryTo))]
pub fn enum_conversions_derive(input: TokenStream) -> TokenStream {
    let enum_ast = syn::parse(input).unwrap();
    impl_conversions(&enum_ast)
}


/// Implements ContainsVariant, GetVariant, SetVariant, and CreateVariantFrom traits
fn impl_conversions(ast: &DeriveInput) -> TokenStream {
    let tera = templates::templater();
    let mut tokens: TokenStream = "".parse().unwrap();

    let name = &ast.ident.to_string();
    let (fullname, lifetimes) = fetch_name_with_generic_params(ast);
    let (
        impl_generics,
        impl_generics_ref,
        where_clause)
        = fetch_impl_generics(ast, ENUM_CONV_LIFETIME,  &lifetimes);
    let field_map = fetch_fields_from_enum(ast);

    tokens.extend::<TokenStream>(create_marker_enums(name, &field_map).parse().unwrap());
    tokens.extend::<TokenStream>(
        impls::impl_get_variant(
            name,
            &fullname,
            &impl_generics,
            &where_clause,
            &field_map,
            &tera,
        )
        .parse()
        .unwrap(),
    );
    tokens.extend::<TokenStream>(
        impls::impl_try_from(
            name,
            &fullname,
            &impl_generics,
            &impl_generics_ref,
            &where_clause,
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
            &impl_generics_ref,
            &where_clause,
            &field_map,
            &tera,
        )
        .parse()
        .unwrap(),
    );
    tokens.extend::<TokenStream>(
        impls::impl_from(&fullname, &where_clause, &impl_generics, &field_map, &tera)
            .parse()
            .unwrap(),
    );
    tokens
}
