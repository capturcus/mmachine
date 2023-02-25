use crate::bits::MValue;

#[test]
fn mvalue_test() {
    let v = MValue::from_u32(1);
    assert_eq!(v.as_string(), "0000000000000001");
    assert_eq!(v.as_u32(), 1);
}

#[test]
fn mvalue_test_2() {
    let v = MValue::from_u32(11111);
    assert_eq!(v.as_string(), "0010101101100111");
    assert_eq!(v.as_u32(), 11111);
}
