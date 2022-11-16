use std::{error::Error, fmt};

/// Custom errors for this crate. Keeps a record of
/// the enum and requested type that produced the error
#[derive(Debug)]
pub struct EnumConversionError {
    name: String,
    requested_type: String,
}

impl EnumConversionError {
    /// Makes the appropriate error message for when get_variant fails
    pub fn new(name: &str, requested_type: &str) -> EnumConversionError {
        EnumConversionError {
            name: name.to_string(),
            requested_type: requested_type.to_string(),
        }
    }
}
impl Error for EnumConversionError {}

impl fmt::Display for EnumConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EnumConversionError :: Active field of enum <{}> is not of type <{}>",
            self.name, self.requested_type,
        )
    }
}

/// This is a helper trait for implementing the [`TryTo`] and
/// [`std::convert::TryFrom`] traits on enums. Is uses marker structs
/// to uniquely identify a type in the enum. This avoids
/// relying on [`std::any::TypeId`] which is limited to types
/// that are `'static`.
trait GetVariant<T, Marker> {
    fn get_variant(self) -> Result<T, EnumConversionError>;
    fn get_variant_ref(&self) -> Result<&T, EnumConversionError>;
    fn get_variant_mut(&mut self) -> Result<&mut T, EnumConversionError>;
}

/// Not all enums can have the [`std::convert::TryFrom`] trait derived
/// on them because of rules around implementing foreign traits on
/// foreign types.
///
/// This trait provides a similar interface that does not have this
/// issue. It closely mimics the [`std::convert::TryInto`] trait.
pub trait TryTo<T> {
    type Error;
    fn try_to(self) -> T;
}
