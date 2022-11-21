use enum_conversion_derive::*;

#[cfg(test)]
mod test_basic {
    use super::*;
    use std::convert::TryFrom;

    #[derive(EnumConversions)]
    #[EnumConv::TryFrom]
    enum Test {
        F1(i32),
        F2(bool),
    }

    #[test]
    fn test_try_from() {
        let test = Test::F1(2);
        let int = test.try_into().unwrap();
        assert_eq!(int, 2);
    }
}
