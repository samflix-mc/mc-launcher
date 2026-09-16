use super::tail;

#[test]
fn la_queue_garde_les_dernieres_lignes() {
    assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail("a", 5), "a");
}
