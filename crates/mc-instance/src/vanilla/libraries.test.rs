use super::*;

fn artifact(size: u64) -> Artifact {
    Artifact {
        path: None,
        sha1: String::new(),
        size,
        url: String::new(),
    }
}

#[test]
fn the_batch_weight_is_the_sum_of_the_announced_sizes() {
    let kept = vec![
        ("a/b/lwjgl.jar".to_string(), artifact(1_200)),
        ("c/d/asm.jar".to_string(), artifact(800)),
    ];

    assert_eq!(weight(&kept), 2_000);
}

#[test]
fn an_empty_batch_weighs_nothing() {
    // All libraries can be excluded by the platform rules. A bar starting
    // from a zero total must not become a division by zero further down —
    // this is where the case is checked.
    assert_eq!(weight(&[]), 0);
}
