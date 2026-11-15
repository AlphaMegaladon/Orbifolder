use orbifolder_core::geometry::{
    DiscreteTorsion, ElementSpec, EmbeddingSpec, GeneratorSpec, Geometry, GeometrySpec, Limits,
    ModelError, SpaceGroup, TheorySpec, TorsionPolicy, embed, invariant_supercharges,
};
use orbifolder_core::math_core::{
    GaugeLattice, GaugeVector, Matrix, Phase, Rational, RightWeight, TorusVector, Vector,
};
use proptest::prelude::*;

fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).unwrap()
}
fn generators() -> [GeneratorSpec; 3] {
    [
        GeneratorSpec {
            order: 2,
            rotation: Matrix::identity(6).unwrap(),
            translation: TorusVector::zero(),
            twist: RightWeight::from_integers([0, 1, 1, 1]),
        },
        GeneratorSpec {
            order: 2,
            rotation: Matrix::new(
                6,
                6,
                (0..36)
                    .map(|k| {
                        if k / 6 != k % 6 {
                            Rational::ZERO
                        } else {
                            Rational::from(if k / 6 < 2 { 1 } else { -1 })
                        }
                    })
                    .collect(),
            )
            .unwrap(),
            translation: TorusVector::zero(),
            twist: Vector::new([Rational::ZERO, Rational::ZERO, r(1, 2), r(-1, 2)]),
        },
        GeneratorSpec {
            order: 2,
            rotation: Matrix::new(
                6,
                6,
                (0..36)
                    .map(|k| {
                        if k / 6 != k % 6 {
                            Rational::ZERO
                        } else {
                            Rational::from(if (2..4).contains(&(k / 6)) { 1 } else { -1 })
                        }
                    })
                    .collect(),
            )
            .unwrap(),
            translation: TorusVector::zero(),
            twist: Vector::new([Rational::ZERO, r(1, 2), Rational::ZERO, r(-1, 2)]),
        },
    ]
}
fn spec() -> GeometrySpec {
    GeometrySpec {
        gram: Matrix::identity(6).unwrap(),
        generators: generators(),
        representatives: None,
        centralizer_presentations: None,
        limits: Limits::default(),
    }
}
fn embedding() -> EmbeddingSpec {
    let mut shifts = std::array::from_fn(|_| GaugeVector::zero());
    shifts[0] = Vector::from_integers([0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
    shifts[1] = Vector::new(std::array::from_fn(|i| {
        if i == 1 {
            r(1, 2)
        } else if i == 2 {
            r(-1, 2)
        } else {
            Rational::ZERO
        }
    }));
    shifts[2] = Vector::new(std::array::from_fn(|i| {
        if i == 0 {
            r(1, 2)
        } else if i == 2 {
            r(-1, 2)
        } else {
            Rational::ZERO
        }
    }));
    EmbeddingSpec {
        label: "unit".into(),
        lattice: GaugeLattice::E8xE8,
        shifts,
        wilson_lines: std::array::from_fn(|_| GaugeVector::zero()),
        torsion: DiscreteTorsion::default(),
    }
}

#[test]
fn exact_three_factor_geometry_and_primitive_fixed_tori() {
    let geometry = Geometry::build(spec()).unwrap();
    assert_eq!(geometry.sectors().len(), 8);
    assert_eq!(
        geometry
            .sectors()
            .iter()
            .map(|s| s.fixed_branes().len())
            .sum::<usize>(),
        98
    );
    for sector in geometry.sectors() {
        for brane in sector.fixed_branes() {
            let expected = if sector.factors()[1..] == [0, 0] {
                6
            } else {
                2
            };
            assert_eq!(brane.locus().dimension(), expected);
            assert_eq!(
                brane.centralizer().invariant_lattice(),
                brane.locus().invariant_lattice()
            );
            for g in brane
                .centralizer()
                .generators(geometry.space_group())
                .unwrap()
            {
                assert!(
                    geometry
                        .space_group()
                        .commutes(brane.element(), &g)
                        .unwrap()
                );
            }
        }
    }
    let spin = geometry
        .space_group()
        .element(ElementSpec {
            factors: [1, 0, 0],
            translation: TorusVector::zero(),
        })
        .unwrap();
    assert_ne!(spin, geometry.space_group().identity());
    assert_eq!(
        geometry
            .space_group()
            .multiply(&spin, &geometry.space_group().identity())
            .unwrap(),
        spin
    );
    let group = embed(geometry, embedding(), TheorySpec::NonSusy).unwrap();
    assert!(group.supercharges().is_empty());
}

#[test]
fn affine_generator_relations_freely_acting_and_integer_lifts() {
    for integer in [false, true] {
        let mut input = spec();
        input.generators[0] = GeneratorSpec {
            order: 1,
            rotation: Matrix::identity(6).unwrap(),
            translation: TorusVector::zero(),
            twist: RightWeight::zero(),
        };
        input.generators[2] = input.generators[0].clone();
        input.generators[1].translation = Vector::new(std::array::from_fn(|i| {
            if i == 0 {
                if integer { Rational::ONE } else { r(1, 2) }
            } else {
                Rational::ZERO
            }
        }));
        let geometry = Geometry::build(input).unwrap();
        assert_eq!(
            geometry.sectors()[1].fixed_branes().len(),
            if integer { 16 } else { 0 }
        );
        let g = geometry
            .space_group()
            .element(ElementSpec {
                factors: [0, 1, 0],
                translation: geometry
                    .space_group()
                    .section_translation([0, 1, 0])
                    .unwrap()
                    .clone(),
            })
            .unwrap();
        let square = geometry.space_group().multiply(&g, &g).unwrap();
        assert_eq!(square.factors(), [0; 3]);
        assert_eq!(
            square.translation().coordinates()[0],
            if integer {
                Rational::from(2)
            } else {
                Rational::ONE
            }
        );
        assert_eq!(
            embed(geometry, embedding(), TheorySpec::NonSusy).unwrap_err(),
            ModelError::UnsupportedEmbedding
        );
    }
}

#[test]
fn reject_bad_metric_character_relations_catalogs_and_limits() {
    let mut bad = spec();
    bad.gram = Matrix::zeros(6, 6).unwrap();
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::InvalidMetric)
    ));
    let mut bad = spec();
    bad.generators[1].rotation = Matrix::identity(6).unwrap();
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::InvalidRotation)
    ));
    let mut bad = spec();
    bad.generators[1].order = 1;
    assert!(Geometry::build(bad).is_err());
    let mut bad = spec();
    bad.generators[1].translation = Vector::new(std::array::from_fn(|i| {
        if i == 0 { r(1, 3) } else { Rational::ZERO }
    }));
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::InvalidRelations)
    ));
    let mut bad = spec();
    bad.limits.point_elements = 7;
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::ResourceLimit)
    ));
    let mut bad = spec();
    bad.limits.fixed_classes = 1;
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::ResourceLimit)
    ));
    let mut bad = spec();
    bad.representatives = Some(vec![vec![]; 8]);
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::IncompleteConstructions)
    ));
    let mut bad = spec();
    bad.centralizer_presentations = Some(vec![vec![]; 98]);
    assert!(matches!(
        Geometry::build(bad),
        Err(ModelError::InvalidPresentation)
    ));
    let group = SpaceGroup::new(
        Matrix::identity(6).unwrap(),
        generators(),
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        group.element(ElementSpec {
            factors: [2, 0, 0],
            translation: TorusVector::zero()
        }),
        Err(ModelError::InvalidElement)
    ));
    assert!(
        group
            .element(ElementSpec {
                factors: [0; 3],
                translation: Vector::new(std::array::from_fn(|_| r(1, 2)))
            })
            .is_err()
    );
    let no_locus = group
        .element(ElementSpec {
            factors: [0; 3],
            translation: Vector::from_integers([1, 0, 0, 0, 0, 0]),
        })
        .unwrap();
    assert!(matches!(
        group.fixed_locus(&no_locus),
        Err(ModelError::NoFixedLocus)
    ));
}

#[test]
fn supersymmetry_is_derived_from_spin_lift() {
    assert_eq!(invariant_supercharges(&[RightWeight::zero()]).len(), 4);
    assert_eq!(
        invariant_supercharges(&[Vector::new([
            Rational::ZERO,
            r(1, 2),
            r(-1, 2),
            Rational::ZERO
        ])])
        .len(),
        2
    );
    assert_eq!(
        invariant_supercharges(&[Vector::new([Rational::ZERO, r(1, 3), r(1, 3), r(-2, 3)])]).len(),
        1
    );
    assert!(invariant_supercharges(&[RightWeight::from_integers([0, 1, 1, 1])]).is_empty());
}

#[test]
fn quantized_torsion_contract_and_non_susy_policy() {
    let geometry = Geometry::build(spec()).unwrap();
    let coefficients = |n, d| {
        let mut entries = vec![Rational::ZERO; 81];
        for (i, j) in [(1, 2), (0, 3), (3, 4)] {
            entries[9 * i + j] = r(n, d);
            entries[9 * j + i] = r(-n, d);
        }
        Matrix::new(9, 9, entries).unwrap()
    };
    let mut input = embedding();
    input.torsion = DiscreteTorsion::new(true, coefficients(1, 2)).unwrap();
    let group = embed(geometry.clone(), input.clone(), TheorySpec::Susy).unwrap();
    let space = geometry.space_group();
    let a = space
        .element(ElementSpec {
            factors: [0, 1, 0],
            translation: TorusVector::zero(),
        })
        .unwrap();
    let b = space
        .element(ElementSpec {
            factors: [0, 0, 1],
            translation: TorusVector::zero(),
        })
        .unwrap();
    let phase = group.projection_torsion_phase(&a, &b).unwrap();
    assert_eq!(phase, Phase::new(r(1, 2)));
    assert_eq!(
        group.projection_torsion_phase(&a, &a).unwrap(),
        Phase::default()
    );
    let ab = space.multiply(&a, &b).unwrap();
    assert_eq!(group.projection_torsion_phase(&a, &ab).unwrap(), phase);
    assert_eq!(group.projection_torsion_phase(&ab, &b).unwrap(), phase);
    let non_susy = embed(geometry.clone(), input, TheorySpec::NonSusy).unwrap();
    assert_eq!(non_susy.torsion_policy(), TorsionPolicy::StoredNotApplied);
    assert_eq!(
        non_susy.projection_torsion_phase(&a, &b),
        Err(ModelError::UnsupportedTorsionProjection)
    );
    let mut input = embedding();
    input.torsion = DiscreteTorsion::new(true, coefficients(1, 5)).unwrap();
    assert!(matches!(
        embed(geometry, input, TheorySpec::Susy),
        Err(ModelError::InvalidTorsion)
    ));
    assert!(DiscreteTorsion::new(true, Matrix::new(9, 9, vec![r(1, 3); 81]).unwrap()).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig { cases:96,
        failure_persistence:Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/geometry_invariants.proptest-regressions")))),
        ..ProptestConfig::default() })]
    #[test]
    fn three_factor_associativity_inverses_and_centralizer(a in prop::array::uniform3(0_u32..2),b in prop::array::uniform3(0_u32..2),c in prop::array::uniform3(0_u32..2),
        x in prop::array::uniform6(-3_i64..3),y in prop::array::uniform6(-3_i64..3),z in prop::array::uniform6(-3_i64..3)) {
        let group=SpaceGroup::new(Matrix::identity(6).unwrap(),generators(),Limits::default()).unwrap();
        let element=|factors,v:[i64;6]|group.element(ElementSpec{factors,translation:Vector::new(v.map(Rational::from))}).unwrap();
        let a=element(a,x);let b=element(b,y);let c=element(c,z);
        prop_assert_eq!(group.multiply(&group.multiply(&a,&b).unwrap(),&c).unwrap(),group.multiply(&a,&group.multiply(&b,&c).unwrap()).unwrap());
        prop_assert_eq!(group.multiply(&a,&group.inverse(&a).unwrap()).unwrap(),group.identity());
        prop_assert_eq!(group.multiply(&group.inverse(&a).unwrap(),&a).unwrap(),group.identity());
        for h in group.centralizer(&a).unwrap().generators(&group).unwrap(){prop_assert!(group.commutes(&a,&h).unwrap());}
    }
}
