use safety_net::circuit::Identifier;

#[test]
fn concat_simple() {
    let id0 = Identifier::new("id0".to_string());
    let id1 = Identifier::new("id1".to_string());
    let id2 = id0 + id1;
    assert_eq!(Identifier::new("id0_id1".to_string()), id2);
}

#[test]
fn concat_simple_w_sliced() {
    let id0 = Identifier::new("id0".to_string());
    let id1 = Identifier::new("id[1]".to_string());
    let id2 = id0 + id1;
    assert_eq!(Identifier::new("\\id0_id_1".to_string()), id2);
    assert!(id2.is_escaped());
}

#[test]
fn concat_simple_w_escaped() {
    let id0 = Identifier::new("id0".to_string());
    let id1 = Identifier::new("id1$".to_string());
    let id2 = id0 + id1;
    assert_eq!(Identifier::new("\\id0_id1$".to_string()), id2);
    assert!(id2.is_escaped());
}

#[test]
fn aig_id() {
    let id0 = Identifier::new("1".to_string());
    assert!(id0.is_escaped());
    let id1 = Identifier::new("inv".to_string());
    let id2 = id0 + id1;
    assert_eq!(Identifier::new("\\1_inv".to_string()), id2);
    assert!(id2.is_escaped());
}
