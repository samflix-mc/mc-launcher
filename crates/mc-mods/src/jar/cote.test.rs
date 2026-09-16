use super::*;

#[test]
fn union_des_cotes() {
    assert_eq!(Side::Client.union(Side::Client), Side::Client);
    assert_eq!(Side::Client.union(Side::Server), Side::Both);
    assert_eq!(Side::Both.union(Side::Client), Side::Both);
    assert!(Side::Both.includes(Side::Server));
    assert!(!Side::Client.includes(Side::Server));
}
