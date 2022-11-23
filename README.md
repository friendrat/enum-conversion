# EnumConversions
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE)

A crate that derives the natural `From` / `TryFrom` traits on enums. The main macros
provide is `#[EnumConversions]` and `#[DeriveTryFrom]`. 

This crate is meant to succeed the [variant_access](https://lib.rs/crates/variant_access) 
crate. It tries to use the more usual `TryFrom` trait rather than crate-native
traits (although this isn't always possible, see below). It also removes the 
need for types in the enum be `'static` and will not compile for generic types
where the definitions could become ambiguous (`variant_access` will compile but
may not provide expected behavior).

## Usage
Given an enum 
```rust
#[EnumConversions]
#[DeriveTryFrom]
enum Enum {
    F1(i32),
    F2(bool),
}
```
will implement the `TryTo` trait (provided by this crate) and the `TryFrom` traits
for each variant in the enum. It will also derive the `From` traits in the
other direction. Without `#[DeriveTryFrom]`, by default, the `TryFrom` trait
is not derived.

If one wishes to derive the `TryFrom` trait only on select variants of the
enum, this can be marked individually instead:
```rust
#[EnumConversions]
enum Enum<U> {
    F1(RefCell<U>),
    #[DeriveTryFrom]
    F2(bool),
}
```

Furthermore, the errors for the `TryTo` / `TryFrom` traits may be configured
by passing the desired error type and a closure mapping the `EnumConversionError`
to said error type as follows:
```rust
use std::error::Error;

#[EnumConvesions(
    Error: Box<dyn Error + 'static>,
    |e| e.to_string().into()
)]
enum Enum<U> {
    F1(RefCell<U>),
    #[DeriveTryFrom]
    F2(bool),
}
```

## Limitations and Gotchas

These should be either validated by the macro, or will lead to a compiler error.
For the former, they can be found in the unit tests inside of `enum-conversion-derive`.
The latter can be found in the `uncompilable_examples` subdirectory of `/tests`.

### Enum variant must contain unambiguous types.
The following types of enums variants do not have an unambiguous type
in each variant
```rust
enum Enum {
    NamedFields{a: bool, b: i32},
    UnnamedField(bool, i32),
    Unit,
}
```
If any of these are present in the enum, the macro will panic.

### No type can be present in more than one variant.

It is not possible to derive `TryFrom<Enum> for bool` where 
```rust
enum Enum {
    F1(bool),
    F2(bool),
}
```
Should the first or second variant be chosen? If a type does not correspond
unambiguously to a single field, the macro will panic or the Rust compiler
will complain of multiple implementations.

A more complicated example of the same phenomenon is
```rust
enum Enum<'a, 'b, U, T> {
    Ref1(&'a U),
    Ref2(&'b T),
}
```
Any blanket implementation of the `TryFrom` trait should also work on the specialized
type `Enum<'a, 'a, bool, bool>`, which is cannot for the above stated reason.
In this case, the macro won't panic, but the compiler will state that multiple
implementations exist and error out.

### Implementing foreign traits on foreign types.

Rust has strong rules about orphan trait implementations, see
[Error Code E0210](https://doc.rust-lang.org/beta/error_codes/E0210.html). 

In particular, implementing a foreign trait on a foreign type is not allowed.
Since `TryFrom` is a foreign trait, it cannot be derived for generic parameters
like so

```rust
#[EnumConversion]
#[DeriveTryFrom]
enum Enum<U> {
    F1(RefCell<U>),
    F2(bool),
}
```

This is why `TryFrom` is not implemented by default and why it can be derived
globally or only for specific variants. The `TryTo` trait is not foreign and
can be used like a `TryInto` replacement instead.