use crate::{
    integer,
    output::{array, compact_anomaly_report, multiplet, object},
    provenance, rational, vector,
};
use orbifolder_core::{
    execution::ModelResult,
    geometry::TheorySpec,
    math_core::Rational,
    physics_engine::{Field, Multiplet, Representation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

type GroupKey<'a> = (
    &'a [Representation],
    &'a [Rational],
    Multiplet,
    &'a Rational,
    Vec<&'a Rational>,
);

type IrrepTable<'a> = BTreeMap<&'a Representation, usize>;

fn components(helicities: &[&Rational]) -> Value {
    array(helicities.iter().map(|helicity| {
        let spin_twice = helicity.i64_parts().and_then(|(num, den)| {
            num.checked_mul(2)
                .filter(|twice| twice % den == 0)
                .map(|twice| (twice / den).unsigned_abs())
        });
        let chirality = if spin_twice.is_some_and(|spin| spin % 2 == 1) {
            Value::from(if *helicity < &Rational::ZERO {
                "left"
            } else {
                "right"
            })
        } else {
            Value::Null
        };
        object([
            ("spin_twice", spin_twice.map_or(Value::Null, Value::from)),
            ("chirality", chirality),
        ])
    }))
}

fn groups<'a>(fields: &'a [Field], tables: &mut [IrrepTable<'a>]) -> Value {
    let mut grouped: BTreeMap<GroupKey<'_>, (Vec<usize>, u128)> = BTreeMap::new();
    for (id, field) in fields.iter().enumerate() {
        let mut helicities = field
            .right_weights()
            .iter()
            .map(|weight| &weight.coordinates()[0])
            .collect::<Vec<_>>();
        helicities.sort_unstable();
        let (ids, states) = grouped
            .entry((
                field.representations(),
                field.u1_charges(),
                field.multiplet(),
                field.mass(),
                helicities,
            ))
            .or_default();
        ids.push(id);
        *states += (field.weight_indices().len() as u128) * (field.right_weights().len() as u128);
    }
    let mut rows = grouped.into_iter().collect::<Vec<_>>();
    rows.sort_unstable_by_key(|(_, (ids, _))| ids[0]);
    array(
        rows.into_iter()
            .map(|((reps, charges, kind, mass, helicities), (ids, states))| {
                object([
                    (
                        "representation_ids",
                        array(reps.iter().zip(tables.iter_mut()).map(|(rep, table)| {
                            let next = table.len();
                            Value::from(*table.entry(rep).or_insert(next))
                        })),
                    ),
                    ("u1_charges", array(charges.iter().map(rational))),
                    ("multiplet", multiplet(kind)),
                    ("components", components(&helicities)),
                    ("mass", rational(mass)),
                    ("field_multiplicity", Value::from(ids.len())),
                    ("field_ids", array(ids.into_iter().map(Value::from))),
                    (
                        "state_multiplicity",
                        u64::try_from(states).map_or_else(|_| integer(&states.into()), Value::from),
                    ),
                ])
            }),
    )
}

pub fn compact_spectrum(result: &ModelResult) -> Value {
    let anomaly = result.anomalies();
    let spectrum = anomaly.map_or_else(|| result.spectrum(), |analysis| analysis.spectrum());
    let gauge = spectrum.basis();
    let generators = gauge.u1_generators();
    let simple_factors = gauge.root_system().factors();
    let mut tables = vec![BTreeMap::new(); simple_factors.len()];
    let fields = groups(spectrum.fields(), &mut tables);
    let tachyons = groups(spectrum.tachyonic().fields(), &mut tables);
    let gram = array(
        generators
            .iter()
            .map(|left| array(generators.iter().map(|right| rational(&left.dot(right))))),
    );
    let factors = array(simple_factors.iter().zip(tables).enumerate().map(
        |(id, (factor, table))| {
            let mut irreps = table.into_iter().collect::<Vec<_>>();
            irreps.sort_unstable_by_key(|&(_, index)| index);
            object([
                ("id", Value::from(id)),
                ("algebra", Value::from(factor.algebra().to_string())),
                (
                    "simple_roots",
                    array(factor.simple_roots().iter().map(vector)),
                ),
                (
                    "irreps",
                    array(irreps.into_iter().map(|(rep, _)| {
                        object([
                            ("dynkin", array(rep.dynkin_labels().iter().map(integer))),
                            ("dimension", integer(rep.dimension())),
                        ])
                    })),
                ),
            ])
        },
    ));
    object([
        ("schema_version", Value::from(1)),
        (
            "theory",
            Value::from(if spectrum.theory() == TheorySpec::Susy {
                "susy"
            } else {
                "non_susy"
            }),
        ),
        (
            "cpt_convention",
            Value::from("explicit_cpt_partners_choose_one_independent_handedness"),
        ),
        (
            "gauge_group",
            object([
                ("factors", factors),
                ("u1_generators", array(generators.iter().map(vector))),
                ("u1_gram", gram),
                (
                    "u1_basis",
                    Value::from(if anomaly.is_some() {
                        "canonical_anomaly"
                    } else {
                        "rref"
                    }),
                ),
            ]),
        ),
        ("massless_count", Value::from(spectrum.fields().len())),
        (
            "tachyon_count",
            Value::from(spectrum.tachyonic().fields().len()),
        ),
        ("fields", fields),
        ("tachyons", tachyons),
        (
            "anomalies",
            anomaly
                .map(|analysis| analysis.report())
                .map(compact_anomaly_report)
                .unwrap_or_else(|| json!({"status":"not_requested"})),
        ),
        (
            "provenance",
            provenance(
                result.provenance(),
                anomaly.map(|analysis| analysis.report()),
            ),
        ),
    ])
}
