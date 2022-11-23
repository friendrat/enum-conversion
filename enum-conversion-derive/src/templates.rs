use super::*;

pub(crate) fn templater() -> Tera {
    let mut tera = Tera::new("/dev/null/*").unwrap();
    tera.add_raw_template("get_variant", GET_VARIANT_TEMPLATE)
        .unwrap();
    tera.add_raw_template("try_from", TRY_FROM_TEMPLATE)
        .unwrap();
    tera.add_raw_template("try_to", TRY_TO_TEMPLATE).unwrap();
    tera.add_raw_template("from", FROM_TEMPLATE).unwrap();
    tera
}

pub(crate) const GET_VARIANT_TEMPLATE: &str = r#"
impl{{ generics }} enum_conversion_traits::GetVariant<{{ Type }}, {{ Marker }} > for {{ fullname }}
{{ Where }} {
    #[allow(unreachable_patterns)]
    fn get_variant(self) -> std::result::Result<{{ Type }}, enum_conversion_traits::EnumConversionError> {
        match self {
            {{ name }}::{{ field }}(inner) => Ok(inner),
            _ => Err(enum_conversion_traits::EnumConversionError::new("{{ fullname }}", "{{ Type }}"))
        }
    }

    #[allow(unreachable_patterns)]
    fn get_variant_ref(&self) -> std::result::Result<&{{ Type }}, enum_conversion_traits::EnumConversionError> {
        match &self {
            {{ name }}::{{ field }}(inner) => Ok(inner),
            _ => Err(enum_conversion_traits::EnumConversionError::new("{{ fullname }}", "{{ Type }}"))
        }
    }

    #[allow(unreachable_patterns)]
    fn get_variant_mut(&mut self) -> std::result::Result<&mut {{ Type }}, enum_conversion_traits::EnumConversionError> {
        match self {
            {{ name }}::{{ field }}(inner) => Ok(inner),
            _  => Err(enum_conversion_traits::EnumConversionError::new("{{ fullname }}", "{{ Type }}"))
        }
    }
}
"#;

pub(crate) const TRY_TO_TEMPLATE: &str = r#"
impl{{ generics }} TryTo<{{ Type }}> for {{ fullname }}
{{ Where }}
{
    type Error = {{ Error }};

    fn try_to(self) -> std::result::Result<{{ Type }}, Self::Error> {
        self.get_variant(){{ Map_Err }}
    }
}

impl{{ generics_ref }} TryTo<&{{ Lifetime }} {{ Type }}> for &{{ Lifetime }} {{ fullname }}
{{ Where }}
{
    type Error = {{ Error }};

    fn try_to(self) -> std::result::Result<&{{ Lifetime }} {{ Type }}, Self::Error> {
        self.get_variant_ref(){{ Map_Err }}
    }
}

impl{{ generics_ref }} TryTo<&{{ Lifetime }} mut {{ Type }}> for &{{ Lifetime }} mut {{ fullname }}
{{ Where }}
{

    type Error = {{ Error }};

    fn try_to(self) -> std::result::Result<&{{ Lifetime }} mut {{ Type }}, Self::Error> {
        self.get_variant_mut(){{ Map_Err }}
    }
}
"#;

pub(crate) const TRY_FROM_TEMPLATE: &str = r#"
impl{{ generics }} TryFrom<{{ fullname }}> for {{ Type }}
{{ Where }}
{
    type Error = {{ Error }};

    fn try_from(value: {{ fullname }}) -> std::result::Result<Self, Self::Error> {
        value.try_to()
    }
}

impl{{ generics_ref }} TryFrom<&{{ Lifetime }} {{ fullname }}> for &{{ Lifetime}} {{ Type }}
{{ Where }}
{
    type Error = {{ Error }};

    fn try_from(value: &{{ Lifetime }} {{ fullname }}) -> std::result::Result<Self, Self::Error> {
        value.try_to()

    }
}

impl{{ generics_ref }} TryFrom<&{{ Lifetime }} mut {{ fullname }}> for &{{ Lifetime }} mut {{ Type }}
{{ Where }}
{
    type Error = {{ Error }};

    fn try_from(value: &{{ Lifetime }} mut {{ fullname }}) -> std::result::Result<Self, Self::Error> {
        value.try_to()
    }
}
"#;

pub(crate) const FROM_TEMPLATE: &str = r#"
impl{{ generics }} From<{{ Type }}> for {{ fullname }}
{{ Where }}
{
    fn from(value: {{ Type }}) -> Self {
        Self::{{ field }}(value)
    }
}
"#;
