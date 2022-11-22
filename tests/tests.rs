

#[cfg(test)]
mod test_advanced_derive_basic {
    use enum_conversion::prelude::advanced::*;
    use std::convert::TryFrom;
    use std::marker::PhantomData;

    /// Simplest kind of enum
    #[derive(Debug, PartialEq, EnumConversions)]
    #[DeriveTryFrom]
    enum Test {
        F1(i32),
        F2(bool),
    }

    /// Enum with multiple lifetimes and multiple
    /// generic parameters.
    #[derive(EnumConversions)]
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

#[cfg(test)]
mod test_advanced_derive_errors {
    use std::error::Error;
    use enum_conversion::prelude::advanced::*;
    use std::convert::TryFrom;

    /// Customize the errors returned in the
    /// TryFrom implementations.
    #[derive(Debug, PartialEq, EnumConversions)]
    #[DeriveTryFrom(
        Error: Box<dyn Error + 'static>,
        |e: EnumConversionError | e.to_string().into(),
    )]
    enum Test {
        F1(i32),
        /// Set the error back to default.
        #[DeriveTryFrom]
        F2(bool),
    }

    #[derive(Debug, PartialEq, EnumConversions)]
    #[TryTo(
        Error: Box<dyn Error + 'static>,
        |e: EnumConversionError | e.to_string().into(),
    )]
    #[DeriveTryFrom]
    enum Test2 {
        F1(i32),
        /// Set the error back to default.
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

    /// Test that the attribute on the F2 variant
    /// restores the error to the default.
    #[test]
    fn test_attribute_default_error() {
        let test: Test = 10_i32.into();
        let int: Result<bool, EnumConversionError> = test.try_into();
        let EnumConversionError{name, requested_type} = int.unwrap_err();
        assert_eq!(name, "Test");
        assert_eq!(requested_type, "bool");
    }

    #[test]
    fn test_custom_try_to() {
        let test: Test2 = 10_i32.into();
        let int: Result<bool, EnumConversionError> = test.try_into();
        let EnumConversionError{name, requested_type} = int.unwrap_err();
        assert_eq!(name, "Test");
        assert_eq!(requested_type, "bool");
    }

}