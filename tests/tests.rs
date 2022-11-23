/// Test that the derived traits behave correctly.
#[cfg(test)]
mod test_derive_basic {
    use enum_conversion::prelude::*;
    use std::convert::TryFrom;
    use std::marker::PhantomData;

    /// Simplest kind of enum
    #[derive(Debug, PartialEq)]
    #[EnumConversions]
    #[DeriveTryFrom]
    enum Test {
        F1(i32),
        F2(bool),
    }

    /// Enum with multiple lifetimes and multiple
    /// generic parameters.
    #[EnumConversions]
    enum Test2<'a, 'b, U, T> {
        F1(&'a U),
        #[DeriveTryFrom]
        F2(bool),
        F3((&'b T, PhantomData<T>)),
    }

    #[test]
    fn test_from() {
        let test: Test = true.into();
        assert_eq!(test, Test::F2(true));
    }

    #[test]
    fn test_try_from() {
        let test = Test::F1(2);
        let int: &i32 = (&test).try_into().expect("Test failed");
        assert_eq!(*int, 2);
        let boolean: Result<bool, EnumConversionError> = test.try_into();
        assert!(boolean.is_err())
    }

    #[test]
    fn test_try_from_mut() {
        let mut test = Test::F1(2);
        let int: &mut i32 = (&mut test).try_into().expect("Test failed");
        assert_eq!(*int, 2);
        *int = 3;
        assert_eq!(test, Test::F1(3));
    }

    #[test]
    fn test_try_from_single_attribute() {
        let test = Test2::<'static, 'static, i32, f64>::F2(false);
        let boolean: bool = test.try_into().expect("Test failed");
        assert!(!boolean);
    }

    #[test]
    fn test_try_to() {
        let int = 2;
        let test = Test2::<'static, 'static, i32, f64>::F1(&2);
        let res = TryTo::<&i32>::try_to(test).expect("Test failed");
        assert_eq!(res, &int);
    }
}

/// Test that errors are configured correctly.
#[cfg(test)]
mod test_derive_errors {
    use enum_conversion::prelude::*;
    use std::convert::TryFrom;
    use std::error::Error;

    /// Customize the errors returned in the
    /// TryFrom implementations.
    #[derive(Debug, PartialEq)]
    #[EnumConversions(
        Error: Box<dyn Error + 'static>,
        |e| e.to_string().into(),
    )]
    #[DeriveTryFrom]
    enum Test {
        F1(i32),
        F2(bool),
    }

    /// Test the attribute on the entire enum
    #[test]
    fn test_custom_error() {
        let test: Test = true.into();
        let int: Result<i32, Box<dyn Error + 'static>> = test.try_into();
        let error = int.unwrap_err().to_string();
        let expected = "EnumConversionError :: Active field of enum <Test> is not of type <i32>";
        assert_eq!(error, expected)
    }

    #[test]
    fn test_custom_try_to() {
        let test: Test = 10_i32.into();
        let int: Result<bool, Box<dyn Error + 'static>> = test.try_into();
        let error = int.unwrap_err().to_string();
        let expected = "EnumConversionError :: Active field of enum <Test> is not of type <bool>";
        assert_eq!(error, expected);
    }
}

/// Tests that the derive macro correctly panics (thereby failing compilation) for the correct
/// cases.
#[cfg(test)]
mod test_compile_failures {
    #[test]
    fn test_uncompilable_examples() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/uncompilable_examples/generics_collision.rs");
        t.compile_fail("tests/uncompilable_examples/foreign_types.rs");
    }
}
