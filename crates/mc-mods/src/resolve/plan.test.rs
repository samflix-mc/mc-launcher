use super::{Plan, Side};
use crate::resolve::fixtures::installed;

#[test]
fn the_plan_filters_by_side() {
    let mut client_only = installed("embeddium", &["embeddium"], &[]);
    client_only.side = Side::Client;
    let plan = Plan {
        mods: vec![installed("jei", &["jei"], &[]), client_only],
        unresolved: Vec::new(),
    };
    assert_eq!(plan.for_side(Side::Server).count(), 1);
    assert_eq!(plan.for_side(Side::Client).count(), 2);
}
