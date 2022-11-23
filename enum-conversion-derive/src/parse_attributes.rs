use std::convert::TryFrom;

use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::__private::TokenStream2;
use syn::{Attribute, Expr};

const ATTR_TRY_FROM: &str = "DeriveTryFrom";

/// The information for each variant
/// in the enum.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct VariantInfo {
    /// The type of the variant.
    pub ty: String,
    /// Indicates if a `TryFrom` trait should be derived
    /// for this variant.
    pub try_from: bool,
}

impl From<&str> for VariantInfo {
    fn from(ty: &str) -> Self {
        VariantInfo {
            ty: ty.to_string(),
            try_from: false,
        }
    }
}

/// The input to the `EnumConversion` macro
/// can configure errors for the
/// `TryTo`/ `TryFrom` traits. In that case,
/// custom error types and a closure for converting
/// to that error type must be given.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ErrorConfig {
    Custom { error_ty: String, map_err: String },
    Default,
}

impl ErrorConfig {
    /// Parse the config into the strings to be
    /// passed into the templates.
    pub(crate) fn to_template(&self) -> (String, String) {
        match self {
            Self::Default => ("EnumConversionError".to_string(), "".into()),
            Self::Custom { error_ty, map_err } => {
                (error_ty.to_string(), format!(".map_err({})", map_err))
            }
        }
    }
}

/// As we extract args one by one,
/// we need to know how to add them
/// to the config.
enum ErrorConfigParam {
    ErrorTy(String),
    MapErr(String),
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self::Default
    }
}

/// Parse attribute macros on the enum and variants.
///
/// Once the attribute macros are processed, they
/// are removed from the AST.
///
/// Returns `true` iff the attribute is `[DeriveTryFrom]`.
pub(crate) fn parse_attrs(attrs: &mut Vec<Attribute>) -> bool {
    let mut derive_try_from = false;
    *attrs = attrs
        .clone()
        .into_iter()
        .filter(|attr| {
            if let Some(prefix) = attr.path.segments.first().map(|seg| seg.ident.to_string()) {
                if prefix == ATTR_TRY_FROM {
                    derive_try_from = true;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        })
        .collect();
    derive_try_from
}

/// Process the arguments passed into the attribute.
/// Panics if they are not of the right format or
/// the wrong number of arguments were passed in.
fn split_args(args: TokenStream2) -> [Expr; 2] {
    let err_str = format!(
        "EnumConversion attribute macros expect either no arguments or exactly two \
         of the form 'Error: Type' and a closure. Found '{}'",
        args,
    );
    // split the args by commas
    let parser = Punctuated::<syn::Expr, Token![,]>::parse_terminated;
    let args = match parser.parse2(args) {
        Ok(args) => args,
        Err(_) => panic!("{}", &err_str),
    };
    // check that we got exactly two args
    if let Ok(res) = <[Expr; 2]>::try_from(args.iter().take(2).cloned().collect::<Vec<Expr>>()) {
        res
    } else {
        panic!("{}", &err_str)
    }
}

/// If an attribute macro is labelled as specifying a custom
/// ErrorConfig and it has arguments, this function parses them.
pub(crate) fn parse_custom_error_config(args: TokenStream2) -> ErrorConfig {
    if args.is_empty() {
        return ErrorConfig::Default;
    }
    let arg_string = args.to_string();
    let [arg1, arg2] = split_args(args);
    let parsed_1 = parse_attr_args(arg1);
    let parsed_2 = parse_attr_args(arg2);
    match (parsed_1, parsed_2) {
        (ErrorConfigParam::ErrorTy(ty), ErrorConfigParam::MapErr(map)) => ErrorConfig::Custom {
            error_ty: ty,
            map_err: map,
        },
        (ErrorConfigParam::MapErr(map), ErrorConfigParam::ErrorTy(ty)) => ErrorConfig::Custom {
            error_ty: ty,
            map_err: map,
        },
        _ => panic!(
            "EnumConversion attribute macros expect either no arguments or exactly two \
            of the form 'Error: Type' and a closure. Found '{}'",
            arg_string,
        ),
    }
}

/// Parse the attribute macros of variants and / or enums as
/// a whole.
fn parse_attr_args(arg: Expr) -> ErrorConfigParam {
    match arg {
        Expr::Type(type_expr) => {
            if type_expr.expr.to_token_stream().to_string() == "Error" {
                ErrorConfigParam::ErrorTy(type_expr.ty.to_token_stream().to_string())
            } else {
                panic!(
                    "EnumConversions expected 'Error: Type', found '{}'",
                    type_expr.to_token_stream(),
                )
            }
        }
        Expr::Closure(closure) => ErrorConfigParam::MapErr(closure.to_token_stream().to_string()),
        _ => panic!(
            "Attribute macros for EnumConversions must either be of the form: \
                     'Error: Type' or a closure."
        ),
    }
}

#[cfg(test)]
mod test_attrs {
    use quote::quote;
    use syn::{parse_str, DeriveInput};

    use super::*;

    /// Test that if no attribute macros are present,
    /// the default error config is generated.
    /// Furthermore, macros unrelated to this crate
    /// are not stripped.
    #[test]
    fn test_no_op() {
        let mut ast: DeriveInput = parse_str(
            r#"
            #[derive(Debug)]
            enum Enum {
                #[random]
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed");
        let ast_clone = ast.clone();
        let attrs = parse_attrs(&mut ast.attrs);
        assert!(!attrs);
        assert_eq!(ast, ast_clone);
    }

    /// Test that the top level macros are stripped when they
    /// are processed.
    #[test]
    fn test_strip_macros() {
        let mut ast: DeriveInput = parse_str(
            r#"
            #[EnumConversion]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                #[DeriveTryFrom]
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");
        assert!(parse_attrs(&mut ast.attrs));
        let expected: DeriveInput = parse_str(
            r#"
            #[EnumConversion]
            enum Enum {
                F1(i64),
                #[DeriveTryFrom]
                F2(bool),
            }
        "#,
        )
        .expect("Test failed");
        assert_eq!(ast, expected);
    }

    /// Test that providing no arguments to
    /// `EnumConversion` returns the default
    /// error config.
    #[test]
    fn test_default_error_config() {
        let args = quote!();
        let error_config = parse_custom_error_config(args);
        assert_eq!(error_config, ErrorConfig::Default);
    }

    /// Test that arguments to the `EnumConversion` trait
    /// get parsed correctly into an `ErrorConfig`.
    #[test]
    fn test_parse_custom_error_config() {
        let args = quote!(Error: std::io::Error, |e| Error::new(
            ErrorKind::Other,
            e.to_string()
        ));

        let error_config = parse_custom_error_config(args);
        let expected = ErrorConfig::Custom {
            error_ty: "std :: io :: Error".to_string(),
            map_err: "| e | Error :: new (ErrorKind :: Other , e . to_string ())".to_string(),
        };
        assert_eq!(error_config, expected);
    }

    #[test]
    #[should_panic(
        expected = "EnumConversion attribute macros expect either no arguments or exactly two of the form 'Error: Type' and a closure. Found 'Error : std :: io :: Error ,'"
    )]
    fn test_wrong_arg_number() {
        let args = quote!(Error: std::io::Error,);

        _ = parse_custom_error_config(args);
    }

    #[test]
    #[should_panic(
        expected = "Attribute macros for EnumConversions must either be of the form: 'Error: Type' or a closure."
    )]
    fn test_non_closure() {
        let args = quote!(Error: std::io::Error, Vec::new);

        _ = parse_custom_error_config(args);
    }

    #[test]
    #[should_panic(
        expected = "EnumConversions expected 'Error: Type', found 'err : std :: io :: Error'"
    )]
    fn test_bad_key() {
        let args = quote!(err: std::io::Error, |e| Error::new(
            ErrorKind::Other,
            e.to_string()
        ));

        _ = parse_custom_error_config(args);
    }

    #[test]
    #[should_panic]
    fn test_non_error_type() {
        let args = quote!(
                Error: || false,
                |e| Error::new(ErrorKind::Other, e.to_string())
        );

        _ = parse_custom_error_config(args);
    }
}
