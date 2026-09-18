use super::super::descriptor::AssetObject;
use super::{count_downloads, weight};
use mc_dl::Fetched;

/// The count separates a first installation — thousands of objects — from a
/// second one, where everything is already there. It's the only explanation
/// we have for the wait: announcing zero when we just pulled down three
/// thousand files would make half an hour look like an anomaly.
#[test]
fn only_the_objects_actually_downloaded_are_counted() {
    let results = vec![
        Ok(Fetched::Downloaded),
        Ok(Fetched::AlreadyPresent),
        Ok(Fetched::Downloaded),
        Ok(Fetched::AlreadyPresent),
    ];

    assert_eq!(count_downloads(results).unwrap(), 2);

    // A second installation downloads nothing, and says so.
    let all_present = vec![Ok(Fetched::AlreadyPresent), Ok(Fetched::AlreadyPresent)];
    assert_eq!(count_downloads(all_present).unwrap(), 0);

    assert_eq!(count_downloads(Vec::new()).unwrap(), 0);
}

/// The first error stops everything: a missing asset makes a texture
/// disappear, not a game that refuses to start, and we'd rather know right
/// away than at the first black screen.
#[test]
fn a_failed_object_stops_the_count() {
    let results = vec![
        Ok(Fetched::Downloaded),
        Err(anyhow::anyhow!("asset abc123: 404")),
        Ok(Fetched::Downloaded),
    ];

    let error = count_downloads(results).expect_err("the failure propagates");
    assert!(format!("{error:#}").contains("abc123"), "{error:#}");
}

/// The batch's weight is the SUM of the announced sizes.
///
/// The function's comment says it's "separated from the loop to be
/// verifiable" — it wasn't: returning zero, or one, left the suite green. Yet
/// this total is what sets the scale of the progress bar. Returning zero
/// would put it at a hundred percent on the first byte; returning one would
/// overflow it three thousand times over.
#[test]
fn the_batch_weight_is_the_sum_of_the_sizes() {
    let objects = vec![
        AssetObject {
            hash: "a".into(),
            size: 1_024,
        },
        AssetObject {
            hash: "b".into(),
            size: 2_048,
        },
        AssetObject {
            hash: "c".into(),
            size: 1,
        },
    ];

    assert_eq!(weight(&objects), 3_073);
    // An empty batch weighs zero — and that's the only case where zero is right.
    assert_eq!(weight(&[]), 0);
}

/// And the sum doesn't overflow on a real index.
///
/// Three thousand asset objects is the order of magnitude of Minecraft 1.21;
/// the total comfortably exceeds what a `u32` could hold.
#[test]
fn the_weight_fits_a_full_index() {
    let objects: Vec<AssetObject> = (0..3_000)
        .map(|i| AssetObject {
            hash: format!("{i:040x}"),
            size: 5_000_000,
        })
        .collect();

    assert_eq!(weight(&objects), 15_000_000_000);
}
