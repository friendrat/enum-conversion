use super::*;
use crate::parse_attributes::{ErrorConfig, VariantInfo};
use crate::parse_enum::ImplGenerics;

/// Implement the helper trait `GetVariant`.
pub(crate) fn impl_get_variant(
    name: &str,
    fullname: &str,
    impl_generics: &ImplGenerics,
    field_map: &HashMap<String, VariantInfo>,
    templater: &Tera,
) -> String {
    let ImplGenerics {
        impl_generics,
        where_clause,
        ..
    } = impl_generics;
    let mut impl_string = String::new();
    for (field, VariantInfo { ty, .. }) in field_map.iter() {
        let mut context = Context::new();
        context.insert("generics", impl_generics);
        context.insert("Type", ty);
        context.insert("Marker", &get_marker(name, field));
        context.insert("fullname", fullname);
        context.insert("name", name);
        context.insert("field", field);
        context.insert("Where", where_clause);
        impl_string.push_str(
            &templater
                .render("get_variant", &context)
                .expect("Failed to render the GetVariant template"),
        );
    }
    impl_string
}

/// Implement the `TryFrom` traits for each type in the
/// enum. Uses the `GetVariant` helper trait and marker structs
/// to avoid generic parameter ambiguity and restrictions
/// to `'static` lifetimes.
pub(crate) fn impl_try_from(
    name: &str,
    fullname: &str,
    impl_generics: &ImplGenerics,
    error_config: &ErrorConfig,
    field_map: &HashMap<String, VariantInfo>,
    templater: &Tera,
) -> String {
    let ImplGenerics {
        impl_generics,
        impl_generics_ref,
        where_clause,
    } = impl_generics;
    let mut impl_string = String::new();
    let (error, _) = error_config.to_template();
    for (field, info) in field_map.iter() {
        if !info.try_from {
            continue;
        };
        let mut where_string = where_clause.to_string();
        let marker_bound = if where_clause.is_empty() {
            format!(
                "where\n {}: GetVariant<{}, {}>",
                fullname,
                &info.ty,
                get_marker(name, field)
            )
        } else {
            format!(
                ",\n {}: GetVariant<{}, {}>",
                fullname,
                &info.ty,
                get_marker(name, field)
            )
        };
        where_string.push_str(&marker_bound);

        let mut context = Context::new();
        context.insert("generics", impl_generics);
        context.insert("generics_ref", impl_generics_ref);
        context.insert("Lifetime", ENUM_CONV_LIFETIME);
        context.insert("Type", &info.ty);
        context.insert("fullname", fullname);
        context.insert("Where", &where_string);
        context.insert("Error", &error);
        impl_string.push_str(
            &templater
                .render("try_from", &context)
                .expect("Failed to render the TryFrom template"),
        );
    }
    impl_string
}

pub(crate) fn impl_try_to(
    name: &str,
    fullname: &str,
    impl_generics: &ImplGenerics,
    error_config: &ErrorConfig,
    field_map: &HashMap<String, VariantInfo>,
    templater: &Tera,
) -> String {
    let ImplGenerics {
        impl_generics,
        impl_generics_ref,
        where_clause,
    } = impl_generics;
    let mut impl_string = String::new();
    let (error, map) = error_config.to_template();
    for (field, info) in field_map.iter() {
        let mut where_string = where_clause.to_string();
        let marker_bound = if where_clause.is_empty() {
            format!(
                "where\n {}: GetVariant<{}, {}>",
                fullname,
                &info.ty,
                get_marker(name, field)
            )
        } else {
            format!(
                ",\n {}: GetVariant<{}, {}>",
                fullname,
                &info.ty,
                get_marker(name, field)
            )
        };
        where_string.push_str(&marker_bound);

        let mut context = Context::new();
        context.insert("generics", impl_generics);
        context.insert("generics_ref", impl_generics_ref);
        context.insert("Type", &info.ty);
        context.insert("Lifetime", ENUM_CONV_LIFETIME);
        context.insert("fullname", fullname);
        context.insert("Where", &where_string);
        context.insert("Error", &error);
        context.insert("Map_Err", &map);
        impl_string.push_str(
            &templater
                .render("try_to", &context)
                .expect("Failed to render the TryFrom template"),
        );
    }
    impl_string
}

pub(crate) fn impl_from(
    fullname: &str,
    impl_generics: &ImplGenerics,
    field_map: &HashMap<String, VariantInfo>,
    templater: &Tera,
) -> String {
    let ImplGenerics {
        impl_generics,
        where_clause,
        ..
    } = impl_generics;
    let mut impl_string = String::new();
    for (field, VariantInfo { ty, .. }) in field_map.iter() {
        let mut context = Context::new();
        context.insert("generics", impl_generics);
        context.insert("Type", ty);
        context.insert("fullname", fullname);
        context.insert("field", field);
        context.insert("Where", where_clause);
        impl_string.push_str(
            &templater
                .render("from", &context)
                .expect("Failed to render the From template"),
        );
    }
    impl_string.parse().unwrap()
}

#[cfg(test)]
mod test_impls {
    use quote::quote;

    use super::*;
    use crate::templates::templater;

    #[test]
    fn test_get_variant() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let name = &ast.ident.to_string();
        let (fullname, lifetimes) = fetch_name_with_generic_params(&mut ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_get_variant(&name, &fullname, &impl_generics, &field_map, &tera);
        let expected = "\nimpl< 'a , T > enum_conversion_traits::GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field > for Enum<'a,T>\nwhere T : Debug {\n    #[allow(unreachable_patterns)]\n    fn get_variant(self) -> std::result::Result<Box < & 'a dyn Into < T > >, enum_conversion_traits::EnumConversionError> {\n        match self {\n            Enum::Field(inner) => Ok(inner),\n            _ => Err(enum_conversion_traits::EnumConversionError::new(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n\n    #[allow(unreachable_patterns)]\n    fn get_variant_ref(&self) -> std::result::Result<&Box < & 'a dyn Into < T > >, enum_conversion_traits::EnumConversionError> {\n        match &self {\n            Enum::Field(inner) => Ok(inner),\n            _ => Err(enum_conversion_traits::EnumConversionError::new(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n\n    #[allow(unreachable_patterns)]\n    fn get_variant_mut(&mut self) -> std::result::Result<&mut Box < & 'a dyn Into < T > >, enum_conversion_traits::EnumConversionError> {\n        match self {\n            Enum::Field(inner) => Ok(inner),\n            _  => Err(enum_conversion_traits::EnumConversionError::new(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_try_from_off() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let name = &ast.ident.to_string();
        let error_config = ErrorConfig::default();
        let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_try_from(
            &name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn test_try_from_on() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            #[EnumConversion(
                Error: Box<dyn Error + 'static>,
                |e| e.to_string().into()
            )]
            #[DeriveTryFrom]
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let name = &ast.ident.to_string();
        let error_config =
            parse_custom_error_config(quote!(Error: Box<dyn Error + 'static>, |e| e
                .to_string()
                .into()));
        let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_try_from(
            &name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        );
        let expected = "\nimpl< 'a , T > TryFrom<Enum<'a,T>> for Box < & 'a dyn Into < T > >\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_from(value: Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryFrom<&'enum_conv Enum<'a,T>> for &'enum_conv Box < & 'a dyn Into < T > >\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_from(value: &'enum_conv Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryFrom<&'enum_conv mut Enum<'a,T>> for &'enum_conv mut Box < & 'a dyn Into < T > >\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_from(value: &'enum_conv mut Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_try_from_partial() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
                #[DeriveTryFrom]
                Other(i64)
            }
        "#,
        )
        .expect("Test failed");
        let name = &ast.ident.to_string();
        let error_config = ErrorConfig::default();
        let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_try_from(
            &name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        );
        let expected = "\nimpl< 'a , T > TryFrom<Enum<'a,T>> for i64\nwhere T : Debug,\n Enum<'a,T>: GetVariant<i64, enum___conversion___Enum::Other>\n{\n    type Error = EnumConversionError;\n\n    fn try_from(value: Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryFrom<&'enum_conv Enum<'a,T>> for &'enum_conv i64\nwhere T : Debug,\n Enum<'a,T>: GetVariant<i64, enum___conversion___Enum::Other>\n{\n    type Error = EnumConversionError;\n\n    fn try_from(value: &'enum_conv Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryFrom<&'enum_conv mut Enum<'a,T>> for &'enum_conv mut i64\nwhere T : Debug,\n Enum<'a,T>: GetVariant<i64, enum___conversion___Enum::Other>\n{\n    type Error = EnumConversionError;\n\n    fn try_from(value: &'enum_conv mut Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.try_to()\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_from() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let (fullname, lifeftimes) = fetch_name_with_generic_params(&ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifeftimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_from(&fullname, &impl_generics, &field_map, &tera);
        let expected = "\nimpl< 'a , T > From<Box < & 'a dyn Into < T > >> for Enum<'a,T>\nwhere T : Debug\n{\n    fn from(value: Box < & 'a dyn Into < T > >) -> Self {\n        Self::Field(value)\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_try_to() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let name = ast.ident.to_string();
        let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
        let error_config = ErrorConfig::default();
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_try_to(
            &name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        );
        let expected = "\nimpl< 'a , T > TryTo<Box < & 'a dyn Into < T > >> for Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = EnumConversionError;\n\n    fn try_to(self) -> std::result::Result<Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant()\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryTo<&'enum_conv Box < & 'a dyn Into < T > >> for &'enum_conv Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = EnumConversionError;\n\n    fn try_to(self) -> std::result::Result<&'enum_conv Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant_ref()\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryTo<&'enum_conv mut Box < & 'a dyn Into < T > >> for &'enum_conv mut Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n\n    type Error = EnumConversionError;\n\n    fn try_to(self) -> std::result::Result<&'enum_conv mut Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant_mut()\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_try_to_custom() {
        let mut ast: DeriveInput = syn::parse_str(
            r#"
          #[EnumConversion(
                Error: Box<dyn Error + 'static>,
                |e| e.to_string().into()
            )]
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#,
        )
        .expect("Test failed");
        let name = ast.ident.to_string();
        let error_config =
            parse_custom_error_config(quote!(Error: Box<dyn Error + 'static>, |e| e
                .to_string()
                .into()));
        let (fullname, lifetimes) = fetch_name_with_generic_params(&ast);
        let impl_generics = fetch_impl_generics(&ast, ENUM_CONV_LIFETIME, &lifetimes);
        let field_map = fetch_fields_from_enum(&mut ast);
        let tera = templater();
        let output = impl_try_to(
            &name,
            &fullname,
            &impl_generics,
            &error_config,
            &field_map,
            &tera,
        );
        let expected = "\nimpl< 'a , T > TryTo<Box < & 'a dyn Into < T > >> for Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_to(self) -> std::result::Result<Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant().map_err(| e | e . to_string () . into ())\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryTo<&'enum_conv Box < & 'a dyn Into < T > >> for &'enum_conv Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_to(self) -> std::result::Result<&'enum_conv Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant_ref().map_err(| e | e . to_string () . into ())\n    }\n}\n\nimpl< 'a , 'enum_conv : 'a , T , > TryTo<&'enum_conv mut Box < & 'a dyn Into < T > >> for &'enum_conv mut Enum<'a,T>\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n\n    type Error = Box < dyn Error + 'static >;\n\n    fn try_to(self) -> std::result::Result<&'enum_conv mut Box < & 'a dyn Into < T > >, Self::Error> {\n        self.get_variant_mut().map_err(| e | e . to_string () . into ())\n    }\n}\n";
        assert_eq!(output, expected);
    }
}
