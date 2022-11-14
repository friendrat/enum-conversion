use super::*;
use super::templates::templater;



/// Implement the helper trait `GetVariant`.
pub(crate) fn impl_get_variant(
    name: &str,
    fullname: &str,
    where_clause: &str,
    impl_generics: &str,
    field_map: &HashMap<String, String>,
    templater: &Tera,
) -> String {
    let mut impl_string = String::new();
    for (field, ty) in field_map.iter() {
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
    impl_string.parse().unwrap()
}

/// Implement the `TryFrom` traits for each type in the
/// enum. Uses the `GetVariant` helper trait and marker structs
/// to avoid generic parameter ambiguity and restrictions
/// to `'static` lifetimes.
pub(crate) fn impl_try_from(
    name: &str,
    fullname: &str,
    where_clause: &str,
    impl_generics: &str,
    field_map: &HashMap<String, String>,
    templater: &Tera,
) -> String {
    let mut impl_string = String::new();
    for (field, ty) in field_map.iter() {
        let mut where_string = where_clause.to_string();
        let marker_bound = if where_clause.is_empty() {
            format!(
                "where\n {}: GetVariant<{}, {}>",
                fullname,
                ty,
                get_marker(name, field)
            )
        } else {
            format!(
                ",\n {}: GetVariant<{}, {}>",
                fullname,
                ty,
                get_marker(name, field)
            )
        };
        where_string.push_str(&marker_bound);
        let mut context = Context::new();
        context.insert("generics", impl_generics);
        context.insert("Type", ty);
        context.insert("fullname", fullname);
        context.insert("Where", &where_string);
        impl_string.push_str(
            &templater
                .render("try_from", &context)
                .expect("Failed to render the TryFrom template"),
        );
    }
    impl_string.parse().unwrap()
}

pub(crate) fn impl_from(
    fullname: &str,
    where_clause: &str,
    impl_generics: &str,
    field_map: &HashMap<String, String>,
    templater: &Tera,
) -> String {
    let mut impl_string = String::new();
    for (field, ty) in field_map.iter() {
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
    use super::*;

    #[test]
    fn test_get_variant() {
        let ast: DeriveInput = syn::parse_str(r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#).expect("Test failed");
        let name = &ast.ident.to_string();
        let fullname = fetch_name_with_generic_params(&ast);
        let (impl_generics, where_clause) = fetch_impl_generics(&ast);
        let field_map = fetch_fields_from_enum(&ast);
        let tera = templater();
        let output = impl_get_variant(&name, &fullname, &where_clause, &impl_generics, &field_map, &tera);
        let expected = "\nimpl< 'a , T > variant_access_traits::GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field > for Enum<'a,T>\nwhere T : Debug {\n    #[allow(unreachable_patterns)]\n    fn get_variant(self) -> std::result::Result<Box < & 'a dyn Into < T > >, variant_access_traits::VariantAccessError> {\n        match self {\n            Enum::Field(inner) => Ok(inner),\n            _ => Err(variant_access_traits::VariantAccessError::wrong_active_field(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n\n    #[allow(unreachable_patterns)]\n    fn get_variant_ref(&self) -> std::result::Result<&Box < & 'a dyn Into < T > >, variant_access_traits::VariantAccessError> {\n        match &self {\n            Enum::Field(inner) => Ok(inner),\n            _ => Err(variant_access_traits::VariantAccessError::wrong_active_field(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n\n    #[allow(unreachable_patterns)]\n    fn get_variant_mut(&mut self) -> std::result::Result<&mut Box < & 'a dyn Into < T > >, variant_access_traits::VariantAccessError> {\n        match self {\n            Enum::Field(inner) => Ok(inner),\n            _  => Err(variant_access_traits::VariantAccessError::wrong_active_field(\"Enum<'a,T>\", \"Box < & 'a dyn Into < T > >\"))\n        }\n    }\n}\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_try_from() {
        let ast: DeriveInput = syn::parse_str(r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#).expect("Test failed");
        let name = &ast.ident.to_string();
        let fullname = fetch_name_with_generic_params(&ast);
        let (impl_generics, where_clause) = fetch_impl_generics(&ast);
        let field_map = fetch_fields_from_enum(&ast);
        let tera = templater();
        let output = impl_try_from(&name, &fullname, &where_clause, &impl_generics, &field_map, &tera);
        let expected = "\nimpl< 'a , T > TryFrom<Enum<'a,T>> for Box < & 'a dyn Into < T > >\nwhere T : Debug,\n Enum<'a,T>: GetVariant<Box < & 'a dyn Into < T > >, enum___conversion___Enum::Field>\n{\n    type Error = Box<dyn Error + 'static>;\n\n    fn try_from(value: Enum<'a,T>) -> std::result::Result<Self, Self::Error> {\n        value.get_variant().map_err(|e| e.to_string().into())\n    }\n}";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_from() {
        let ast: DeriveInput = syn::parse_str(r#"
            enum Enum<'a, T>
            where
                T: Debug
            {
                Field(Box<&'a dyn Into<T>>),
            }
        "#).expect("Test failed");
        let fullname = fetch_name_with_generic_params(&ast);
        let (impl_generics, where_clause) = fetch_impl_generics(&ast);
        let field_map = fetch_fields_from_enum(&ast);
        let tera = templater();
        let output = impl_from(&fullname, &where_clause, &impl_generics, &field_map, &tera);
        let expected = "\nimpl< 'a , T > From<Box < & 'a dyn Into < T > >> for Enum<'a,T>\nwhere T : Debug\n{\n\n    fn from(value: Box < & 'a dyn Into < T > >) -> Self {\n        Self::Field(value)\n    }\n}\n";
        assert_eq!(output, expected);
    }
}