use std::collections::HashMap;
use std::fmt::Write as _;

use quote::ToTokens;
use syn::__private::Span;
use syn::{Data, GenericParam, Lifetime, LifetimeDef, Token};

use super::*;
use crate::parse_attributes::{parse_attrs, VariantInfo};

/// This functions determines the name of the enum with generic
/// params attached.
///
/// # Example
/// ```
/// use std::fmt::Debug;
/// enum Enum<'a, T: 'a + Debug, const X: usize> {
///     F1(&'a T),
///     F2([T; X])
/// }
/// ```
/// This function should return `(Enum<'a, T, X>, vec!['a])`
pub fn fetch_name_with_generic_params(ast: &DeriveInput) -> (String, Vec<String>) {
    let mut param_string = String::new();
    let mut lifetimes = vec![];
    for param in ast.generics.params.iter() {
        let next = match param {
            syn::GenericParam::Type(type_) => type_.ident.to_token_stream(),
            syn::GenericParam::Lifetime(life_def) => {
                let lifetime = life_def.lifetime.to_token_stream();
                lifetimes.push(lifetime.to_string());
                lifetime
            }
            syn::GenericParam::Const(constant) => constant.ident.to_token_stream(),
        };
        _ = write!(param_string, "{},", next);
    }
    param_string.pop();
    if !param_string.is_empty() {
        (format!("{}<{}>", ast.ident, param_string), lifetimes)
    } else {
        (ast.ident.to_string(), lifetimes)
    }
}

/// The generic arguments and lifetimes that must
/// be added to trait implementations.
pub struct ImplGenerics {
    /// The generic params inherited from the decorated
    /// type.
    pub impl_generics: String,
    /// For returning references, an extra lifetime with
    /// appropriate bounds must be used in addition to
    /// the generics from the type.
    pub impl_generics_ref: String,
    /// The where clause with trait bounds from the decorated
    /// type.
    pub where_clause: String,
}

/// This fetches the generics for impl blocks on the traits
/// and the where clause.
///
/// # Example:
/// ```
/// use std::fmt::Debug;
/// pub enum Enum<'a, T: Debug, U>
///where
///     U: Into<T>
/// {
///     F1(&'a T),
///     F2(U)
/// }
/// ```
/// returns
/// `
/// ImplGenerics {
///     impl_generics: "<T: Debug, U>",
///     impl_generics_ref: "<'a, 'enum_conv: 'a, T: Debug, U>",
///     where_clause: "where U: Into<T>",
/// }
/// `
///
///
/// For traits the return references, the lifetime of the reference must be bound
/// by lifetimes in the definition of the enum.
pub fn fetch_impl_generics(ast: &DeriveInput, lifetime: &str, bounds: &[String]) -> ImplGenerics {
    let mut generics = ast.generics.clone();
    let mut generics_ref = generics.clone();
    generics_ref
        .params
        .push(GenericParam::Lifetime(bound_lifetime(lifetime, bounds)));

    let where_clause = generics
        .where_clause
        .take()
        .map(|w| w.to_token_stream().to_string());
    ImplGenerics {
        impl_generics: generics.to_token_stream().to_string(),
        impl_generics_ref: generics_ref.to_token_stream().to_string(),
        where_clause: where_clause.unwrap_or_default(),
    }
}

/// Given a lifetime and a list of other lifetimes, creates
/// the bound that states the input lifetime cannot outlive
/// the lifetimes in the list.
pub fn bound_lifetime(lifetime: &str, bounds: &[String]) -> syn::LifetimeDef {
    let mut lifetime_def = LifetimeDef::new(Lifetime::new(lifetime, Span::call_site()));
    lifetime_def.colon_token = if bounds.is_empty() {
        Some(Token![:](Span::call_site()))
    } else {
        None
    };
    lifetime_def.bounds = bounds
        .iter()
        .map(|lifetime| Lifetime::new(lifetime, Span::call_site()))
        .collect();
    lifetime_def
}

/// Fetches the name of each variant in the enum and
/// maps it to a string representation of its type.
///
/// Also performs validation for unsupported enum types.
/// These include:
///  * Enums with multiple variants of the same type.
///  * Enums with variants with multiple or named fields.
///  * Enums with unit variants.
///
/// Will panic if the input type is not an enum.
pub(crate) fn fetch_fields_from_enum(ast: &mut DeriveInput) -> HashMap<String, VariantInfo> {
    let derive_globally = parse_attrs(&mut ast.attrs);
    if let Data::Enum(data) = &mut ast.data {
        let mut num_fields: usize = 0;
        let mut types = data
            .variants
            .iter_mut()
            .map(|var| match &var.fields {
                syn::Fields::Unnamed(field_) => {
                    if field_.unnamed.len() != 1 {
                        panic!(
                            "Can only derive for enums whose types do \
                             not contain multiple fields."
                        );
                    }
                    let var_ty = field_
                        .unnamed
                        .iter()
                        .next()
                        .unwrap()
                        .ty
                        .to_token_stream()
                        .to_string();
                    let var_name = var.ident.to_token_stream().to_string();
                    let var_info = VariantInfo {
                        ty: var_ty,
                        try_from: parse_attrs(&mut var.attrs) || derive_globally,
                    };
                    num_fields += 1;
                    (var_info, var_name)
                }
                syn::Fields::Named(_) => {
                    panic!("Can only derive for enums whose types do not have named fields.")
                }
                syn::Fields::Unit => {
                    panic!("Can only derive for enums who don't contain unit types as variants.")
                }
            })
            .collect::<HashMap<VariantInfo, String>>();
        let types: HashMap<String, VariantInfo> = types.drain().map(|(k, v)| (v, k)).collect();
        if types.keys().len() != num_fields {
            panic!("Cannot derive for enums with more than one field with the same type.")
        }
        types
    } else {
        panic!("Can only derive for enums.")
    }
}

/// Creates a marker enum for each field in the enum
/// under a new module.
///
/// Used to identify types in the enum and disambiguate
/// generic parameters.
pub(crate) fn create_marker_enums(name: &str, types: &HashMap<String, VariantInfo>) -> String {
    let mut piece = format!(
        "#[allow(non_snake_case)]\n mod enum___conversion___{}",
        name
    );
    piece.push_str("{ ");
    for field in types.keys() {
        _ = write!(piece, "pub(crate) enum {}{{}}", field);
    }
    piece.push('}');
    piece
}

/// Get the fully qualified name of the marker struct
/// associated with an enum variant.
pub fn get_marker(name: &str, field: &str) -> String {
    format!("enum___conversion___{}::{}", name, field)
}

#[cfg(test)]
mod test_parsers {

    use super::*;

    const ENUM: &str = r#"
            enum Enum<'a, 'b, T, U: Debug>
            where T: Into<U>, U: 'a
            {
                #[help]
                Array([u8; 20]),
                BareFn(fn(&'a usize) -> bool),
                Macro(typey!()),
                Path(<Vec<&'a mut T> as IntoIterator>::Item),
                Ptr(*const u8),
                Tuple((&'b i64, bool)),
                Slice([u8]),
                Trait(Box<&dyn Into<U>>),
            }
        "#;

    /// Test that we support all possible types in an enum,
    /// and that we get the names of the field correctly.
    /// We also check that attribute macros are supported.
    #[test]
    fn test_parse_fields_and_types() {
        let mut ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let fields = fetch_fields_from_enum(&mut ast);
        let expected: HashMap<String, VariantInfo> = HashMap::from([
            ("Array".to_string(), "[u8 ; 20]".into()),
            ("BareFn".to_string(), "fn (& 'a usize) -> bool".into()),
            ("Macro".to_string(), "typey ! ()".into()),
            (
                "Path".to_string(),
                "< Vec < & 'a mut T > as IntoIterator > :: Item".into(),
            ),
            ("Ptr".to_string(), "* const u8".into()),
            ("Slice".to_string(), "[u8]".into()),
            ("Trait".to_string(), "Box < & dyn Into < U > >".into()),
            ("Tuple".to_string(), "(& 'b i64 , bool)".into()),
        ]);
        assert_eq!(expected, fields);
    }

    #[test]
    fn test_global_try_from_config() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed");
        let fields = fetch_fields_from_enum(&mut ast);
        let expected: HashMap<String, VariantInfo> = HashMap::from([
            (
                "F1".to_string(),
                VariantInfo {
                    ty: "i64".to_string(),
                    try_from: true,
                },
            ),
            (
                "F2".to_string(),
                VariantInfo {
                    ty: "bool".to_string(),
                    try_from: true,
                },
            ),
        ]);
        assert_eq!(fields, expected);
    }

    #[test]
    fn test_try_from_local_config() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum {
                F1(i64),
                #[DeriveTryFrom]
                F2(bool),
            }
        "#,
        )
        .expect("Test failed");
        let fields = fetch_fields_from_enum(&mut ast);
        let expected: HashMap<String, VariantInfo> = HashMap::from([
            ("F1".to_string(), "i64".into()),
            (
                "F2".to_string(),
                VariantInfo {
                    ty: "bool".to_string(),
                    try_from: true,
                },
            ),
        ]);
        assert_eq!(fields, expected);
    }

    #[test]
    fn test_generics_and_bounds() {
        let ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let (_, lifetimes) = fetch_name_with_generic_params(&ast);
        let ImplGenerics {
            impl_generics,
            impl_generics_ref,
            where_clause,
        } = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        assert_eq!(impl_generics, "< 'a , 'b , T , U : Debug >");
        assert_eq!(
            impl_generics_ref,
            "< 'a , 'b , 'enum_conv : 'a + 'b , T , U : Debug , >"
        );
        assert_eq!(where_clause, "where T : Into < U > , U : 'a");
    }

    #[test]
    fn test_get_name_with_generics() {
        let ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let (name, lifetimes) = fetch_name_with_generic_params(&ast);
        assert_eq!(name, "Enum<'a,'b,T,U>");
        assert_eq!(lifetimes, vec![String::from("'a"), String::from("'b")]);
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums.")]
    fn test_panic_on_struct() {
        let mut ast = syn::parse_str("pub struct Struct;").expect("Test failed");
        _ = fetch_fields_from_enum(&mut ast);
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums whose types do not have named fields.")]
    fn test_panic_on_field_with_named_types() {
        let mut ast = syn::parse_str(
            r#"
            enum Enum {
                F{a: i64},
            }
        "#,
        )
        .expect("Test failed");
        _ = fetch_fields_from_enum(&mut ast);
    }

    #[test]
    #[should_panic(
        expected = "Cannot derive for enums with more than one field with the same type."
    )]
    fn test_multiple_fields_same_type() {
        let mut ast = syn::parse_str(
            r#"
        enum Enum {
            F1(u64),
            F2(u64),
        }
        "#,
        )
        .expect("Test failed");
        _ = fetch_fields_from_enum(&mut ast);
    }

    #[test]
    #[should_panic(
        expected = "Can only derive for enums whose types do not contain multiple fields."
    )]
    fn test_multiple_types_in_field() {
        let mut ast = syn::parse_str(
            r#"
            enum Enum {
                Field(i64, bool),
            }
        "#,
        )
        .expect("Test failed");
        _ = fetch_fields_from_enum(&mut ast);
    }

    #[test]
    #[should_panic(
        expected = "Can only derive for enums who don't contain unit types as variants."
    )]
    fn test_unit_type() {
        let mut ast = syn::parse_str(
            r#"
            enum Enum {
                Some(bool),
                None,
            }
        "#,
        )
        .expect("Test failed");
        _ = fetch_fields_from_enum(&mut ast);
    }

    /// If an enum has no fields, this derive macro will be a no-op
    #[test]
    fn test_harmless() {
        let mut ast = syn::parse_str(r#"enum Enum{ }"#).expect("Test failed");
        let fields = fetch_fields_from_enum(&mut ast);
        assert!(fields.is_empty())
    }

    #[test]
    fn test_create_marker_structs() {
        let mut ast = syn::parse_str(
            r#"
            enum Enum {
                F1(u64)
            }
        "#,
        )
        .expect("Test failed.");
        let fields = fetch_fields_from_enum(&mut ast);
        let output = create_marker_enums(&ast.ident.to_string(), &fields);
        assert_eq!(
            output,
            "#[allow(non_snake_case)]\n mod enum___conversion___Enum{ pub(crate) enum F1{}}"
        );
    }
}
