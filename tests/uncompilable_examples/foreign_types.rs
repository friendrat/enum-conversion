/// We cannot derive a foreign trait (`TryFrom`) on
/// a foreign type (as implied by the generic parameter `U`).
///
/// This is due to error [E0210](https://doc.rust-lang.org/error_codes/E0210.html)
/// regarding orphan rules for trait implementations.
///
/// The solution is to use the `TryTo` trait provided by this crate.

use enum_conversion::prelude::*;

#[EnumConversions]
#[DeriveTryFrom]
enum Enum<'a, U> {
    F1(&'a U),
    F2(bool),
}

fn main() {

}