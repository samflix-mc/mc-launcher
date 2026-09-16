use super::*;

#[test]
fn un_canal_plus_stable_que_la_limite_est_accepte() {
    assert!(Channel::Release.allowed_by(Channel::Beta));
    assert!(Channel::Beta.allowed_by(Channel::Beta));
    assert!(!Channel::Alpha.allowed_by(Channel::Beta));
    assert!(!Channel::Beta.allowed_by(Channel::Release));
}
