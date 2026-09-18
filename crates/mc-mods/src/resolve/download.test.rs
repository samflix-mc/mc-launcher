use super::*;

fn job(size: u64) -> Job {
    Job {
        key: (Origin::Modrinth, "jei".to_string()),
        url: String::new(),
        file_name: String::new(),
        sum: None,
        size,
    }
}

#[test]
fn the_batch_weight_is_the_sum_of_the_announced_sizes() {
    assert_eq!(weight(&[job(4_000_000), job(1_500_000)]), 5_500_000);
}

#[test]
fn a_mod_with_no_published_size_counts_as_zero() {
    // CurseForge without a key doesn't publish a size. The total becomes a
    // floor: the bar will speed up at the end, which beats refusing to show
    // one at all.
    assert_eq!(weight(&[job(0), job(2_000)]), 2_000);
}

#[test]
fn an_empty_batch_weighs_nothing() {
    // Everything is already cached: nothing to download, and above all no
    // division by zero in whatever ends up displaying a percentage.
    assert_eq!(weight(&[]), 0);
}
