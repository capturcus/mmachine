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

#[test]
fn mvalue_test_add() {
    let v = MValue::from_u32(69);
    let v2 = MValue::from_u32(420);
    v.add(&v2);
    assert_eq!(v.as_u32(), 489);

    let v = MValue::from_u32(6719);
    let v2 = MValue::from_u32(7877);
    v.add(&v2);
    assert_eq!(v.as_u32(), 14596);
}

#[test]
fn mvalue_test_sub() {
    let v = MValue::from_u32(69);
    let v2 = MValue::from_u32(420);
    v2.sub(&v);
    assert_eq!(v2.as_u32(), 351);
    
    let v = MValue::from_u32(19937);
    let v2 = MValue::from_u32(9377);
    v.sub(&v2);
    assert_eq!(v.as_u32(), 10560);
}

#[test]
fn mvalue_test_mul() {
    let v = MValue::from_u32(69);
    let v2 = MValue::from_u32(42);
    v.mul(&v2);
    assert_eq!(v.as_u32(), 2898);
}