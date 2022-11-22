use std::convert::TryFrom;

use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::{Attribute, Expr};

const ATTR_TRY_TO: &str = "TryTo";
const ATTR_TRY_FROM: &str = "DeriveTryFrom";

/// The information for each variant
/// in the enum.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct VariantInfo {
    /// The type of the variant.
    pub ty: String,
    /// The attribute macros on the variant.
    pub attrs: VariantAttrs,
}

impl From<&str> for VariantInfo {
    fn from(ty: &str) -> Self {
        VariantInfo {
            ty: ty.to_string(),
            attrs: Default::default(),
        }
    }
}

/// Represents that attributes on variants
/// that we process.
///
/// Each field can provide an optional map_err
/// closure to be used in the `TryTo` and `TryFrom`
/// trait implementations.
#[derive(Hash, PartialEq, Debug, Clone, Eq)]
pub(crate) struct VariantAttrs {
    pub try_from: Option<ErrorConfig>,
    pub try_to: ErrorConfig,
}

impl Default for VariantAttrs {
    fn default() -> Self {
        VariantAttrs {
            try_from: None,
            try_to: ErrorConfig::Default,
        }
    }
}

/// Attributes can configure errors for the
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
/// The global args are defined on the enum struct (or is replaced
/// with a default if not attribute macro is used). Individual
/// variants may overwrite this default.
pub(crate) fn parse_attrs(attrs: &[Attribute], mut global: VariantAttrs) -> VariantAttrs {
    for attr in attrs.iter() {
        if let Some(prefix) = attr.path
                .segments
                .first()
                .map(|seg| seg.ident.to_string())
        {
            if prefix ==  ATTR_TRY_FROM {
                if !attr.tokens.is_empty() {
                    global.try_from = Some(parse_custom_error_config(attr));
                } else {
                    global.try_from = Some(ErrorConfig::Default);
                }
            }
            if prefix  == ATTR_TRY_TO {
                if !attr.tokens.is_empty() {
                    global.try_to = parse_custom_error_config(attr);
                } else {
                    global.try_to = ErrorConfig::Default;
                }
            }
        }
    }
    global
}

/// Split the token stream inside an attribute macro into
/// two separate args.
fn split_args(tokens: proc_macro2::TokenStream) -> [Expr; 2] {
    let err_str = format!(
        "EnumConversion attribute macros expect either no arguments or exactly two \
         of the form 'Error: Type' and a closure. Found '{}'",
        tokens,
    );
    // strip the outer delimiters
    let args = match tokens.into_iter().next().unwrap() {
        proc_macro2::TokenTree::Group(group) => group.stream(),
        _ => panic!("{}", &err_str),
    };
    // split the args by commas
    let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
    let parsed: Punctuated<Expr, Token![,]> = parser.parse2(args).expect(&err_str);
    // check that we got exactly two args
    if let Ok(res) = <[Expr; 2]>::try_from(parsed.iter().take(2).cloned().collect::<Vec<Expr>>()) {
        res
    } else {
        panic!("{}", &err_str)
    }
}

/// If an attribute macro is labelled as specifying a custom
/// ErrorConfig and it has arguments, this function parses them.
fn parse_custom_error_config(attr: &Attribute) -> ErrorConfig {
    let [arg1, arg2] = split_args(attr.tokens.clone());
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
            attr.tokens,
        ),
    }
}

/// Parse the attribute macros of variants and /or enums as
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
    use super::*;
    use syn::{parse_str, DeriveInput};

    /// Test that if no attribute macros are present,
    /// the default error config is generated.
    #[test]
    fn parse_global_default() {
        let ast: DeriveInput = parse_str(
            r#"
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed");
        let attrs = parse_attrs(&ast.attrs, Default::default());
        assert_eq!(
            attrs,
            VariantAttrs {
                try_from: None,
                try_to: Default::default()
            }
        )
    }

    #[test]
    fn parse_global_tryto() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                Error: std::io::Error,
                |e| Error::new(ErrorKind::Other, e.to_string())
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");
        let attrs = parse_attrs(&ast.attrs, Default::default());
        let expected = VariantAttrs {
            try_from: Some(Default::default()),
            try_to: ErrorConfig::Custom {
                error_ty: "std :: io :: Error".to_string(),
                map_err: "| e | Error :: new (ErrorKind :: Other , e . to_string ())".to_string(),
            },
        };
        assert_eq!(attrs, expected);
    }

    #[test]
    fn test_overwite_config() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                Error: std::io::Error,
                |e| Error::new(ErrorKind::Other, e.to_string())
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");
        let global = VariantAttrs {
            try_from: Some(ErrorConfig::Custom {
                error_ty: "test".into(),
                map_err: "test".into(),
            }),
            try_to: ErrorConfig::Custom {
                error_ty: "test".into(),
                map_err: "test".into(),
            },
        };
        let attrs = parse_attrs(&ast.attrs, global);
        let expected = VariantAttrs {
            try_from: Some(Default::default()),
            try_to: ErrorConfig::Custom {
                error_ty: "std :: io :: Error".to_string(),
                map_err: "| e | Error :: new (ErrorKind :: Other , e . to_string ())".to_string(),
            },
        };
        assert_eq!(attrs, expected);
    }

    #[test]
    #[should_panic(
        expected = "EnumConversion attribute macros expect either no arguments or exactly two of the form 'Error: Type' and a closure. Found '(Error : std :: io :: Error)'"
    )]
    fn test_wrong_arg_number() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                Error: std::io::Error
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");

        _ = parse_attrs(&ast.attrs, Default::default());
    }

    #[test]
    #[should_panic(
        expected = "Attribute macros for EnumConversions must either be of the form: 'Error: Type' or a closure."
    )]
    fn test_non_closure() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                Error: std::io::Error,
                Vec::new
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");

        _ = parse_attrs(&ast.attrs, Default::default());
    }

    #[test]
    #[should_panic(
        expected = "EnumConversions expected 'Error: Type', found 'err : std :: io :: Error'"
    )]
    fn test_bad_key() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                err: std::io::Error,
                Vec::new
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");

        _ = parse_attrs(&ast.attrs, Default::default());
    }

    #[test]
    #[should_panic]
    fn test_non_error_type() {
        let ast: DeriveInput = parse_str(
            r#"
            #[TryTo(
                Error: || false,
                Vec::new
              )]
            #[DeriveTryFrom]
            enum Enum {
                F1(i64),
                F2(bool),
            }
        "#,
        )
        .expect("Test failed.");

        _ = parse_attrs(&ast.attrs, Default::default());
    }
}
