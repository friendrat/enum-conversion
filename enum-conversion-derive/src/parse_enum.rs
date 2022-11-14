use super::*;
use quote::ToTokens;
use std::collections::HashMap;
use syn::Data;

/// This functions determines the name of the enum with generic
/// params attached.
///
/// # Example
/// ```
/// enum Enum<'a, T: 'a + Debug, const X: usize> {
///     F1(T),
///     F2(X)
/// }
/// ```
/// This function should return `Enum<'a, T, X>`
pub fn fetch_name_with_generic_params(ast: &DeriveInput) -> String {
    let mut param_string = String::new();
    for param in ast.generics.params.iter() {
        let next = match param {
            syn::GenericParam::Type(type_) => type_.ident.to_token_stream(),
            syn::GenericParam::Lifetime(life_def) => life_def.lifetime.to_token_stream(),
            syn::GenericParam::Const(constant) => constant.ident.to_token_stream(),
        };
        param_string.push_str(&format!("{},", next));
    }
    param_string.pop();
    if !param_string.is_empty() {
        format!("{}<{}>", ast.ident, param_string)
    } else {
        ast.ident.to_string()
    }
}

/// This fetches the generics for impl blocks on the traits
/// and the where clause.
///
/// # Example:
/// ```
/// pub enum Enum<T: Debug, U>
///where
///     U: Into<T>
/// {
///     F1(T),
///     F2(U)
/// }
/// ```
/// returns `("<T: Debug, U>", "where U: Into<T>")`.
pub fn fetch_impl_generics(ast: &DeriveInput) -> (String, String) {
    let mut generics = ast.generics.clone();
    let where_clause = generics
        .where_clause
        .take()
        .map(|w| w.to_token_stream().to_string());
    (
        generics.to_token_stream().to_string(),
        where_clause.unwrap_or_default(),
    )
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
pub fn fetch_fields_from_enum(ast: &DeriveInput) -> HashMap<String, String> {
    if let Data::Enum(data) = &ast.data {
        let mut num_fields: usize = 0;
        let mut types = data.variants
            .iter()
            .map(|var| match &var.fields {
                syn::Fields::Unnamed(field_) => {
                    if field_.unnamed.len() != 1 {
                        panic!(
                            "Can only derive for enums whose types do \
                             not contain multiple fields."
                        );
                    }
                    let field_ty = field_
                        .unnamed
                        .iter()
                        .next()
                        .unwrap()
                        .ty
                        .to_token_stream()
                        .to_string();
                    let field_name = var.ident.to_token_stream().to_string();
                    num_fields += 1;
                    (field_ty, field_name)
                }
                syn::Fields::Named(_) => {
                    panic!("Can only derive for enums whose types do not have named fields.")
                }
                syn::Fields::Unit => {
                    panic!("Can only derive for enums who don't contain unit types as variants.")
                }
            })
            .collect::<HashMap<String, String>>();
        let types: HashMap<String, String> = types.drain().map(|(k, v)| (v, k)).collect();
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
pub fn create_marker_enums(name: &str, types: &HashMap<String, String>) -> String {
    let mut piece = format!("#[allow(non_snake_case]\n mod enum___conversion___{}", name);
    piece.push_str("{ ");
    for field in types.keys() {
        piece.push_str(&format!("pub(crate) enum {}{{}}", field));
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
            enum Enum<'a, T, U: Debug>
            where T: Into<U>, U: 'a
            {
                #[help]
                Array([u8; 20]),
                BareFn(fn(&'a usize) -> bool),
                Macro(typey!()),
                Path(<Vec<&'a mut T> as IntoIterator>::Item),
                Ptr(*const u8),
                Tuple((&'a i64, bool)),
                Slice([u8]),
                Trait(Box<&dyn Into<U>>),
            }
        "#;

    /// Test that we support all possible types in an enum,
    /// and that we get the names of the field correctly.
    /// We also check that attribute macros are supported.
    #[test]
    fn test_parse_fields_and_types() {
        let ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let fields = fetch_fields_from_enum(&ast);
        let expected = HashMap::from([
            ("Array".to_string(), "[u8 ; 20]".to_string()),
            ("BareFn".to_string(), "fn (& 'a usize) -> bool".to_string()),
            ("Macro".to_string(), "typey ! ()".to_string()),
            ("Path".to_string(), "< Vec < & 'a mut T > as IntoIterator > :: Item".to_string()),
            ("Ptr".to_string(), "* const u8".to_string()),
            ("Slice".to_string(), "[u8]".to_string()),
            ("Trait".to_string(), "Box < & dyn Into < U > >".to_string()),
            ("Tuple".to_string(), "(& 'a i64 , bool)".to_string()),
        ]);
        assert_eq!(expected, fields);
    }

    #[test]
    fn test_generics_and_bounds() {
        let ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let (generics, where_clause) = fetch_impl_generics(&ast);
        assert_eq!(generics, "< 'a , T , U : Debug >");
        assert_eq!(where_clause, "where T : Into < U > , U : 'a");
    }

    #[test]
    fn test_get_name_with_generics() {
        let ast: DeriveInput = syn::parse_str(ENUM).expect("Test failed.");
        let name = fetch_name_with_generic_params(&ast);
        assert_eq!(name, "Enum<'a,T,U>")
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums.")]
    fn test_panic_on_struct() {
        let ast = syn::parse_str(
            "pub struct Struct;"
        ).expect("Test failed");
        _ = fetch_fields_from_enum(&ast);
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums whose types do not have named fields.")]
    fn test_panic_on_field_with_named_types() {
        let ast = syn::parse_str(r#"
            enum Enum {
                F{a: i64},
            }
        "#).expect("Test failed");
        _ = fetch_fields_from_enum(&ast);
    }

    #[test]
    #[should_panic(expected = "Cannot derive for enums with more than one field with the same type.")]
    fn test_multiple_fields_same_type() {
        let ast = syn::parse_str(r#"
        enum Enum {
            F1(u64),
            F2(u64),
        }
        "#).expect("Test failed");
        _ = fetch_fields_from_enum(&ast);
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums whose types do not contain multiple fields.")]
    fn test_multiple_types_in_field() {
        let ast = syn::parse_str(r#"
            enum Enum {
                Field(i64, bool),
            }
        "#).expect("Test failed");
        _ = fetch_fields_from_enum(&ast);
    }

    #[test]
    #[should_panic(expected = "Can only derive for enums who don't contain unit types as variants.")]
    fn test_unit_type() {
        let ast = syn::parse_str(r#"
            enum Enum {
                Some(bool),
                None,
            }
        "#).expect("Test failed");
        _ = fetch_fields_from_enum(&ast);
    }

    /// If an enum has no fields, this derive macro will be a no-op
    #[test]
    fn test_harmless() {
        let ast = syn::parse_str(r#"enum Enum{ }"#).expect("Test failed");
        let fields = fetch_fields_from_enum(&ast);
        assert!(fields.is_empty())
    }

    #[test]
    fn test_create_marker_structs() {
        let ast = syn::parse_str(r#"
            enum Enum {
                F1(u64)
            }
        "#).expect("Test failed.");
        let fields = fetch_fields_from_enum(&ast);
        let output = create_marker_enums(&ast.ident.to_string(), &fields );
        assert_eq!(output, "#[allow(non_snake_case]\n mod enum___conversion___Enum{ pub(crate) enum F1{}}");
    }
}