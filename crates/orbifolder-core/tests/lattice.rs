mod common;
use orbifolder_core::math_core::{
    DnClass, GaugeLattice, Rational, Vector, dn_class, e8_roots, so8_class,
};
use proptest::prelude::*;
use std::collections::BTreeSet;

#[test]
fn native_lattice_and_spin32_observations() {
    let native = common::json("math_oracle.json");
    for variant in ["susy", "non_susy"] {
        let p = &native["primitives"][variant];
        let roots = p["spin32_roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| common::vector::<16>(x, 2))
            .collect::<BTreeSet<_>>();
        assert_eq!(roots, GaugeLattice::D16Plus.roots().into_iter().collect());
        for case in p["lattice_cases"].as_array().unwrap() {
            let weight = common::vector(&case["weight"], 3);
            assert_eq!(
                GaugeLattice::E8xE8.contains(&weight),
                case["e8xe8"].as_bool().unwrap()
            );
            assert_eq!(
                GaugeLattice::D16Plus.contains(&weight),
                case["spin32"].as_bool().unwrap()
            );
        }
        let a = common::number(&p["large_arithmetic"]["a"], 1);
        let b = common::number(&p["large_arithmetic"]["b"], 1);
        assert!(a.uses_bigint());
        assert_eq!(&a + &b, common::number(&p["large_arithmetic"]["sum"], 1));
        assert_eq!(a * b, common::number(&p["large_arithmetic"]["product"], 1));
    }
}

#[test]
fn native_so8_classes_with_explicit_legacy_mapping() {
    let data = common::json("math_oracle.json");
    for case in data["primitives"]["non_susy"]["so8_cases"]
        .as_array()
        .unwrap()
    {
        let class = so8_class(&common::vector(&case["weight"], 4));
        let s = match class {
            Some(DnClass::Vector) => 1,
            Some(DnClass::Spinor) => 2,
            _ => 0,
        };
        let c = match class {
            Some(DnClass::Vector) => 1,
            Some(DnClass::Cospinor) => 3,
            _ => 0,
        };
        assert_eq!(s, case["s"].as_i64().unwrap());
        assert_eq!(c, case["c"].as_i64().unwrap());
    }
    for case in data["primitives"]["susy"]["so8_cases"].as_array().unwrap() {
        let class = so8_class(&common::vector(&case["weight"], 4));
        let legacy = match class {
            Some(DnClass::Vector) => 1,
            Some(DnClass::Spinor) => 2,
            _ => 0,
        };
        assert_eq!(legacy, case["ns_r"].as_i64().unwrap());
    }
    assert_eq!(so8_class(&Vector::zero()), Some(DnClass::Zero));
}

#[test]
fn root_counts_membership_cosets_and_orders() {
    assert_eq!(e8_roots().len(), 240);
    for (lattice, n) in [
        (GaugeLattice::E8xE8, 480),
        (GaugeLattice::D16Plus, 480),
        (GaugeLattice::D8xD8, 224),
    ] {
        let roots = lattice.roots();
        assert_eq!(roots.len(), n);
        assert_eq!(roots.iter().collect::<BTreeSet<_>>().len(), n);
        assert!(
            roots
                .iter()
                .all(|r| lattice.contains(r) && r.norm_squared() == Rational::from(2))
        );
        let v = roots[0].scale(&Rational::new(1, 3).unwrap());
        assert_eq!(lattice.coset_order(&v), 3.into());
        assert!(lattice.same_coset(&v, &v.add(&roots[1])));
    }
    let half = Vector::new(std::array::from_fn(|_| Rational::new(1, 2).unwrap()));
    assert!(GaugeLattice::D16Plus.contains(&half));
    assert!(!GaugeLattice::D8xD8.contains(&half));
    assert_eq!(GaugeLattice::D8xD8.coset_order(&half), 2.into());
    let mixed = Vector::new(std::array::from_fn(|i| {
        if i == 0 {
            Rational::new(1, 2).unwrap()
        } else {
            Rational::ZERO
        }
    }));
    assert_eq!(GaugeLattice::E8xE8.coset_order(&mixed), 4.into());
    let all = GaugeLattice::E8xE8.roots();
    let breaking = Vector::from_integers([0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0])
        .scale(&Rational::new(1, 2).unwrap());
    // The full non-SUSY breaking shift is integral, with half-integral spinor products.
    let breaking = breaking.scale(&Rational::from(2));
    assert_eq!(GaugeLattice::E8xE8.surviving_roots(&[breaking]).len(), 224);
    assert_eq!(all.len(), 480);
}

proptest! {
    #[test]
    fn even_dn_discriminant_group_matches_vector_addition(a in prop::array::uniform8(-10i64..10),b in prop::array::uniform8(-10i64..10),ha in any::<bool>(),hb in any::<bool>()) {
        let make=|values:[i64;8],half:bool| Vector::new(values.map(|n|Rational::from(n)+Rational::new(i128::from(half),2).unwrap()));
        let a=make(a,ha); let b=make(b,hb);
        let class=|v:&Vector<8>| dn_class(v.coordinates()).unwrap().unwrap();
        prop_assert_eq!(class(&a).compose(class(&b)),class(&a.add(&b)));
    }
}
