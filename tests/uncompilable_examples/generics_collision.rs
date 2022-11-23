/// In this test, no foreign traits are derived and
/// no foreign types are in the decorated enum.
///
/// However, if the two generic parameters are set
/// to be equal, the enum will contain variants with
/// identical types. It will the be impossible to implement
/// the `TryTo` trait and the rust compiler will complain
/// accordingly about conflicting implementations.
use enum_conversion::prelude::*;

struct Local<T>(T);

/// Note that
/// ```
/// enum Enum<U> {
///     F1(Local<U>),
///     F2(Local<bool>),
/// }
/// ```
/// would suffer the same problems as `U` could
/// be given the type `bool`.
#[EnumConversions]
enum Enum<U, T> {
    F1(Local<U>),
    F2(Local<T>)
}



fn main() {

}