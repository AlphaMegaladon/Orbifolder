use orbifolder_core::math_core::Rational;
use orbifolder_core::{
    analysis::Computation,
    anomalies::{AnomalyAnalysis, AnomalyCheck, AnomalyReport, AnomalyTraces},
    execution::{ComputeError, ModelResult, Provenance},
    geometry::ModelError,
    geometry::{LocalElement, SpaceGroupElement},
    math_core::{Matrix, Vector},
    physics_engine::{
        Excitation, Field, HalfState, MassShell, Multiplet, Oscillator, ProjectedState, Spectrum,
        TachyonSearch,
    },
    susy::{CouplingReport, RuleCharge, Superpotential},
};
use serde_json::{Number, Value, json};
pub(crate) fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}
pub(crate) fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}
pub fn rational(r: &Rational) -> Value {
    let (num, den) = r.i64_parts().map_or_else(
        || (integer(&r.numerator()), integer(&r.denominator())),
        |(num, den)| (Value::from(num), Value::from(den)),
    );
    object([("num", num), ("den", den)])
}
pub fn integer(n: &num_bigint::BigInt) -> Value {
    if let Ok(value) = i64::try_from(n) {
        return Value::from(value);
    }
    Value::Number(
        n.to_string()
            .parse::<Number>()
            .expect("BigInt is an integer token"),
    )
}
fn ratios(v: &[Rational]) -> Value {
    Value::Array(v.iter().map(rational).collect())
}
pub fn vector<const N: usize>(v: &Vector<N>) -> Value {
    ratios(v.coordinates())
}
pub fn matrix(m: &Matrix) -> Value {
    Value::Array(
        (0..m.rows())
            .map(|i| {
                Value::Array(
                    (0..m.cols())
                        .map(|j| rational(m.get(i, j).unwrap()))
                        .collect(),
                )
            })
            .collect(),
    )
}
fn element(e: &SpaceGroupElement) -> Value {
    json!({"factors":e.factors(),"translation":vector(e.translation())})
}
fn local(e: &LocalElement) -> Value {
    json!({"element":element(e.element()),"shift":vector(e.shift()),"twist":vector(e.twist())})
}
fn excitation(e: &Excitation) -> Value {
    json!({"oscillators":e.indices(),"energy":rational(e.energy()),"target":rational(e.target()),"excited":e.is_excited()})
}
fn shells<const N: usize>(s: &[MassShell<N>]) -> Value {
    Value::Array(s.iter().enumerate().map(|(id,s)|json!({"id":id,"excitation":excitation(s.excitation()),"weights":s.weights().iter().map(vector).collect::<Vec<_>>()})).collect())
}
fn modes(o: &[Oscillator]) -> Value {
    Value::Array(o.iter().enumerate().map(|(id,o)|json!({"id":id,"direction":o.direction(),"conjugate":o.conjugate(),"frequency":rational(o.frequency()),"energy":rational(&o.energy())})).collect())
}
fn half(h: &HalfState) -> Value {
    json!({"shell":h.shell(),"weight_indices":h.indices(),"phases":ratios(h.phases())})
}
fn halves(h: &[HalfState]) -> Value {
    Value::Array(
        h.iter()
            .enumerate()
            .map(|(id, h)| json!({"id":id,"state":half(h)}))
            .collect(),
    )
}
fn states(s: &[ProjectedState]) -> Value {
    Value::Array(s.iter().enumerate().map(|(id,s)|json!({"id":id,"left":half(s.left()),"right":half(s.right()),"gamma":ratios(s.gamma()),"oscillator_contribution":vector(s.oscillator_contribution()),"mass":rational(s.mass())})).collect())
}
pub fn field(id: usize, f: &Field) -> Value {
    let kind = multiplet(f.multiplet());
    json!({"id":id,"source":f.source(),"element":element(f.element()),"weight_indices":f.weight_indices(),"highest_weight":vector(f.highest_weight()),"representations":f.representations().iter().map(|r|json!({"algebra":r.algebra().to_string(),"dynkin":r.dynkin_labels().iter().map(integer).collect::<Vec<_>>(),"dimension":integer(r.dimension())})).collect::<Vec<_>>(),"u1_charges":ratios(f.u1_charges()),"multiplet":kind,"right_weights":f.right_weights().iter().map(vector).collect::<Vec<_>>(),"origin_classes":f.origin_classes().iter().map(|c|format!("{c:?}")).collect::<Vec<_>>(),"spacetime_classes":f.spacetime_classes().iter().map(|c|format!("{c:?}")).collect::<Vec<_>>(),"scalar_charge":f.scalar_charge().map(vector),"left_excitation":excitation(f.left_excitation()),"right_excitation":excitation(f.right_excitation()),"gamma":ratios(f.gamma()),"oscillator_contribution":vector(f.oscillator_contribution()),"mass":rational(f.mass())})
}
pub(crate) fn multiplet(kind: Multiplet) -> Value {
    match kind {
        Multiplet::Susy(m) => json!({"family":"susy","kind":format!("{m:?}")}),
        Multiplet::Particle(p) => json!({"family":"particle","kind":format!("{p:?}")}),
        Multiplet::Tachyon { excited, conjugate } => {
            json!({"family":"tachyon","excited":excited,"conjugate":conjugate})
        }
    }
}
pub fn spectrum(s: &Spectrum) -> Value {
    json!({"theory":format!("{:?}",s.theory()),"gauge_group":{"roots":s.basis().root_system().roots().iter().map(vector).collect::<Vec<_>>(),"factors":s.basis().root_system().factors().iter().map(|f|json!({"algebra":f.algebra().to_string(),"simple_roots":f.simple_roots().iter().map(vector).collect::<Vec<_>>(),"cartan":matrix(&f.cartan_matrix().expect("validated Lie factor"))})).collect::<Vec<_>>(),"u1_generators":s.basis().u1_generators().iter().map(vector).collect::<Vec<_>>()},"fields":s.fields().iter().enumerate().map(|(id,f)|field(id,f)).collect::<Vec<_>>(),"tachyons":s.tachyonic().fields().iter().enumerate().map(|(id,f)|field(id,f)).collect::<Vec<_>>()})
}
fn traces(t: &AnomalyTraces) -> Value {
    let n = t.u1_count();
    let cubic = array((0..n).map(|left| {
        array((0..n).map(|middle| {
            array((0..n).map(|right| {
                rational(
                    t.abelian_cubic(left, middle, right)
                        .expect("valid U(1) index"),
                )
            }))
        }))
    }));
    object([
        ("gravitational", ratios(t.gravitational())),
        ("mixed", array(t.mixed().iter().map(|row| ratios(row)))),
        ("gauge_cubic", ratios(t.gauge_cubic())),
        ("abelian_cubic", cubic),
        (
            "su2_doublets",
            array(
                t.su2_doublets()
                    .iter()
                    .map(|x| x.as_ref().map_or(Value::Null, integer)),
            ),
        ),
        ("su2_parity", json!(t.su2_parity())),
    ])
}
pub fn status<T>(c: &Computation<T>, encode: impl Fn(&T) -> Value) -> Value {
    match c {
        Computation::Computed(v) => json!({"status":"computed","value":encode(v)}),
        Computation::NotApplicable(reason) => json!({"status":"not_applicable","reason":reason}),
        Computation::Unsupported(reason) => json!({"status":"unsupported","reason":reason}),
    }
}
pub fn anomalies(a: &AnomalyAnalysis) -> Value {
    let mut data = anomaly_report(a.report());
    data["raw"] = traces(a.report().raw());
    data["spectrum"] = spectrum(a.spectrum());
    data
}
pub fn anomaly_report(a: &AnomalyReport) -> Value {
    anomaly_report_with(a, false)
}
pub(crate) fn compact_anomaly_report(a: &AnomalyReport) -> Value {
    anomaly_report_with(a, true)
}
fn anomaly_report_with(r: &AnomalyReport, compact: bool) -> Value {
    let rotation = r.rotation();
    let fi = r.fi();
    object([
        ("status", Value::from("computed")),
        ("is_anomaly_free", Value::from(r.is_anomaly_free())),
        (
            "canonical",
            if compact {
                sparse_traces(r.canonical())
            } else {
                traces(r.canonical())
            },
        ),
        (
            "rotation",
            object([
                (
                    "generators",
                    array(rotation.generators().iter().map(vector)),
                ),
                ("transformation", matrix(rotation.transformation())),
                ("gram", matrix(rotation.gram())),
                ("trace_vector", vector(rotation.trace_vector())),
                ("anomalous_index", json!(rotation.anomalous_index())),
            ]),
        ),
        ("levels", ratios(r.non_abelian_levels())),
        (
            "fi",
            object([
                ("legacy_d0", rational(fi.legacy_d0())),
                ("norm_squared", rational(fi.anomalous_norm_squared())),
                ("trace_over24", rational(fi.gs_trace_over24())),
                (
                    "susy_coefficient",
                    status(fi.supersymmetric_coefficient(), rational),
                ),
            ]),
        ),
        (
            "checks",
            array(r.checks().iter().map(|check| {
                object([
                    ("check", check_kind(check.kind())),
                    (
                        "result",
                        status(check.status(), |passed| Value::from(*passed)),
                    ),
                    ("actual", check.actual().map_or(Value::Null, rational)),
                    ("expected", check.expected().map_or(Value::Null, rational)),
                ])
            })),
        ),
    ])
}
fn check_kind(check: &AnomalyCheck) -> Value {
    match check {
        AnomalyCheck::GaugeCubic { factor } => json!({"kind":"gauge_cubic","factor":factor}),
        AnomalyCheck::Witten { factor } => json!({"kind":"witten","factor":factor}),
        AnomalyCheck::MixedUniversal { u1, factor } => {
            json!({"kind":"mixed_universal","u1":u1,"factor":factor})
        }
        AnomalyCheck::AbelianUniversal { indices } => {
            json!({"kind":"abelian_universal","indices":indices})
        }
        AnomalyCheck::TracelessGravity { u1 } => json!({"kind":"traceless_gravity","u1":u1}),
        AnomalyCheck::TracelessMixed { u1, factor } => {
            json!({"kind":"traceless_mixed","u1":u1,"factor":factor})
        }
        AnomalyCheck::IndependentAbelianDirections => {
            json!({"kind":"independent_abelian_directions"})
        }
        AnomalyCheck::FiNormalization => json!({"kind":"fi_normalization"}),
    }
}
fn sparse_traces(t: &AnomalyTraces) -> Value {
    let size = t.u1_count();
    let mut cubic = Vec::new();
    for left in 0..size {
        for middle in left..size {
            for right in middle..size {
                let value = t
                    .abelian_cubic(left, middle, right)
                    .expect("valid U(1) index");
                if !value.is_zero() {
                    cubic.push(array([
                        Value::from(left),
                        Value::from(middle),
                        Value::from(right),
                        rational(value),
                    ]));
                }
            }
        }
    }
    object([
        ("gravitational", ratios(t.gravitational())),
        ("mixed", array(t.mixed().iter().map(|row| ratios(row)))),
        ("gauge_cubic", ratios(t.gauge_cubic())),
        (
            "abelian_cubic",
            object([
                ("size", Value::from(size)),
                ("symmetric_nonzero", Value::Array(cubic)),
            ]),
        ),
        (
            "su2_doublets",
            array(
                t.su2_doublets()
                    .iter()
                    .map(|value| value.as_ref().map_or(Value::Null, integer)),
            ),
        ),
        ("su2_parity", json!(t.su2_parity())),
    ])
}
fn anomaly_checks(report: &AnomalyReport) -> Value {
    let factors = report.non_abelian_levels().len();
    let u1s = report.canonical().u1_count();
    let mut gauge = vec![b'-'; factors];
    let mut witten = vec![b'-'; factors];
    let mut mixed = vec![b'-'; factors * u1s];
    let mut traceless_mixed = vec![b'-'; factors * u1s];
    let mut gravity = vec![b'-'; u1s];
    let mut cubic = vec![b'-'; u1s * (u1s + 1) * (u1s + 2) / 6];
    let mut independent = b'-';
    let mut fi = b'-';
    for check in report.checks() {
        let value = match check.status() {
            Computation::Computed(true) => b'P',
            Computation::Computed(false) => b'F',
            Computation::NotApplicable(_) => b'N',
            Computation::Unsupported(_) => b'U',
        };
        match *check.kind() {
            AnomalyCheck::GaugeCubic { factor } => gauge[factor] = value,
            AnomalyCheck::Witten { factor } => witten[factor] = value,
            AnomalyCheck::MixedUniversal { u1, factor } => mixed[u1 * factors + factor] = value,
            AnomalyCheck::TracelessMixed { u1, factor } => {
                traceless_mixed[u1 * factors + factor] = value
            }
            AnomalyCheck::TracelessGravity { u1 } => gravity[u1] = value,
            AnomalyCheck::AbelianUniversal {
                indices: [left, middle, right],
            } => {
                let preceding_left = (0..left)
                    .map(|index| {
                        let remaining = u1s - index;
                        remaining * (remaining + 1) / 2
                    })
                    .sum::<usize>();
                let preceding_middle = (left..middle).map(|index| u1s - index).sum::<usize>();
                cubic[preceding_left + preceding_middle + right - middle] = value;
            }
            AnomalyCheck::IndependentAbelianDirections => independent = value,
            AnomalyCheck::FiNormalization => fi = value,
        }
    }
    let statuses = |values| Value::String(String::from_utf8(values).expect("ASCII check statuses"));
    object([
        ("factor_count", Value::from(factors)),
        ("u1_count", Value::from(u1s)),
        ("gauge_cubic", statuses(gauge)),
        ("witten", statuses(witten)),
        ("mixed_universal", statuses(mixed)),
        ("traceless_mixed", statuses(traceless_mixed)),
        ("traceless_gravity", statuses(gravity)),
        ("abelian_universal", statuses(cubic)),
        (
            "independent_abelian_directions",
            statuses(vec![independent]),
        ),
        ("fi_normalization", statuses(vec![fi])),
    ])
}
pub fn coupling(c: &CouplingReport) -> Value {
    let i = c.invariant();
    json!({"fields":i.monomial().fields(),"degree":i.monomial().degree(),"charge_sum":ratios(i.charge_sum()),"tensor_singlets":integer(i.tensor_singlets()),"polynomial_singlets":integer(i.polynomial_singlets()),"gauge_invariant":i.gauge_invariant(),"native_gauge_invariant":i.native_gauge_invariant(),"native_pre":c.native_precoupling(),"native_allowed":c.native_allowed(),"polynomial_allowed":c.polynomial_allowed(),"point_group":c.point_group(),"space_group":c.space_group(),"native_fixed_torus":c.native_fixed_torus(),"gamma_sums":ratios(c.gamma_sums()),"gamma":status(c.gamma(),|b|json!(b)),"discrete":c.discrete().iter().map(|check|{let r=check.rule();let (kind,op)=match r.operator(){RuleCharge::R(op)=>("r",ratios(op.as_ref())),RuleCharge::NonR(op)=>("non_r",ratios(op.as_ref()))};json!({"kind":kind,"order":r.order(),"target":rational(r.target()),"operator":op,"charge_sum":rational(check.charge_sum()),"passed":check.passed()})}).collect::<Vec<_>>()})
}
pub fn superpotential(w: &Superpotential) -> Value {
    json!({"status":"computed","max_degree":w.max_degree(),"field_pool":w.field_pool(),"candidates":w.candidates().iter().map(coupling).collect::<Vec<_>>(),"allowed":w.allowed().map(|(id,_)|id).collect::<Vec<_>>()})
}
pub fn result(r: &ModelResult) -> Value {
    let mut data = snapshot(r);
    data["provenance"] = provenance(r.provenance(), r.anomalies().map(AnomalyAnalysis::report));
    data
}
fn snapshot(r: &ModelResult) -> Value {
    let solved = r.solved();
    let model = solved.model();
    let geometry = model.geometry();
    let space = geometry.space_group();
    json!({"schema_version":1,"model":{"label":model.label(),"theory":format!("{:?}",model.theory()),"lattice":format!("{:?}",model.lattice())},"geometry":{"orders":space.orders(),"gram":matrix(space.gram()),"generators":space.generators().iter().map(|g|json!({"order":g.order,"rotation":matrix(&g.rotation),"translation":vector(&g.translation),"twist":vector(&g.twist)})).collect::<Vec<_>>(),"sectors":geometry.sectors().iter().enumerate().map(|(si,s)|json!({"id":si,"factors":s.factors(),"branes":s.fixed_branes().iter().enumerate().map(|(bi,b)|json!({"id":bi,"element":element(b.element()),"point":vector(b.locus().point()),"invariant_lattice":b.locus().invariant_lattice().iter().map(vector).collect::<Vec<_>>(),"centralizer":b.centralizer().generators(space).expect("validated centralizer").iter().map(element).collect::<Vec<_>>(),"centralizer_presentation":b.centralizer_presentation().iter().map(element).collect::<Vec<_>>()})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"wilson_relations":{"orders":geometry.wilson_relations().orders().iter().map(|n|n.as_ref().map(integer)).collect::<Vec<_>>(),"aliases":geometry.wilson_relations().aliases(),"relations":geometry.wilson_relations().relations().iter().map(vector).collect::<Vec<_>>()}},"orbifold_group":{"elements":model.elements().iter().map(local).collect::<Vec<_>>(),"centralizers":model.centralizers().iter().map(|c|c.iter().map(local).collect::<Vec<_>>()).collect::<Vec<_>>(),"supercharges":model.supercharges().iter().map(vector).collect::<Vec<_>>(),"modular_checks":model.modular_checks().iter().map(|c|json!({"indices":c.indices(),"prefactor":integer(c.prefactor()),"value":rational(c.value())})).collect::<Vec<_>>()},"sectors":solved.sectors().iter().enumerate().map(|(si,s)|json!({"id":si,"factors":s.factors(),"twist":vector(s.twist()),"a_left":rational(s.modes().vacuum().left()),"a_right":rational(s.modes().vacuum().right()),"left_modes":modes(s.modes().left()),"right_modes":modes(s.modes().right()),"right_shells":shells(s.right_shells()),"right_movers":halves(s.right_movers()),"tachyon_shells":shells(s.tachyon_shells()),"tachyon_movers":halves(s.tachyon_movers()),"tachyon_masses":ratios(s.tachyon_masses()),"branes":s.branes().iter().enumerate().map(|(bi,b)|json!({"id":bi,"element":element(b.element()),"left_shells":shells(b.left_shells()),"left_movers":halves(b.left_movers()),"states":states(b.states()),"tachyon_shells":b.tachyon_shells().iter().map(|s|shells(s)).collect::<Vec<_>>(),"tachyons":states(b.tachyons())})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"spectrum":spectrum(r.spectrum()),"anomalies":r.anomalies().map(anomalies).unwrap_or_else(||json!({"status":"not_requested"})),"susy":status(r.susy(),|_|json!({"available":true,"max_degree":3}))})
}
pub fn error(e: &ComputeError) -> Value {
    let category = match e {
        ComputeError::Model(_) => "model",
        ComputeError::Physics(_) => "physics",
        ComputeError::Anomalies(_) => "anomalies",
        ComputeError::Susy(_) => "susy",
        ComputeError::InvalidOptions => "input_error",
        ComputeError::Filtered(_) => "filtered",
    };
    json!({"status":"error","category":category,"message":e.to_string()})
}
pub fn scan_error(e: &ComputeError, stage: &str) -> Value {
    use orbifolder_core::{anomalies::AnomalyError, physics_engine::PhysicsError, susy::SusyError};
    let (category, code) = match e {
        ComputeError::InvalidOptions => ("input_error", "invalid_compute_options"),
        ComputeError::Model(ModelError::InvalidEmbeddingOrder) => {
            ("physics_invalid", "embedding_order")
        }
        ComputeError::Model(ModelError::WilsonRelation) => ("physics_invalid", "wilson_relation"),
        ComputeError::Model(ModelError::ModularInvariance { .. }) => {
            ("physics_invalid", "modular_invariance")
        }
        ComputeError::Model(ModelError::InvalidTorsion) => ("physics_invalid", "invalid_torsion"),
        ComputeError::Model(ModelError::UnsupportedEmbedding) => {
            ("unsupported", "unsupported_embedding")
        }
        ComputeError::Model(ModelError::UnsupportedGaugeLattice) => {
            ("unsupported", "unsupported_lattice")
        }
        ComputeError::Model(ModelError::UnsupportedTorsionProjection)
        | ComputeError::Physics(PhysicsError::Model(ModelError::UnsupportedTorsionProjection)) => {
            ("unsupported", "unsupported_torsion_projection")
        }
        ComputeError::Model(ModelError::ResourceLimit)
        | ComputeError::Physics(
            PhysicsError::ResourceLimit | PhysicsError::Model(ModelError::ResourceLimit),
        )
        | ComputeError::Susy(SusyError::ResourceLimit) => ("resource_limit", "solver_limit"),
        ComputeError::Physics(PhysicsError::UnsupportedSupermultiplet)
        | ComputeError::Anomalies(AnomalyError::UnsupportedMultiplet) => {
            ("unsupported", "unsupported_multiplet")
        }
        _ => ("internal_error", "computation_failed"),
    };
    let mut data = json!({"status":"error","category":category,"code":code,"stage":stage,"message":e.to_string()});
    if let ComputeError::Model(ModelError::ModularInvariance { left, right, value }) = e {
        data["slots"] = json!([left, right]);
        data["value"] = rational(value);
    }
    data
}

pub fn provenance(proof: &Provenance, anomaly: Option<&AnomalyReport>) -> Value {
    let mode = match proof.tachyon_search {
        Some(TachyonSearch::Complete) => "complete",
        Some(TachyonSearch::LegacyWindow) => "legacy",
        _ => "none",
    };
    let tachyon_status = if proof.minimum_mass.is_some() {
        "found"
    } else if proof.tachyon_excluded_by_theory {
        "excluded_by_theory"
    } else if proof.tachyon_complete {
        "complete"
    } else if mode == "none" {
        "not_requested"
    } else {
        "incomplete"
    };
    object([
        ("stage", Value::from(proof.stage)),
        (
            "canonical_spectrum_assembled",
            Value::from(proof.canonical_spectrum_assembled),
        ),
        (
            "massless_sectors_solved",
            Value::from(proof.massless_sectors_solved),
        ),
        (
            "representations_assembled",
            Value::from(proof.representations_assembled),
        ),
        ("susy_view_computed", Value::from(proof.susy_view_computed)),
        ("tachyon_search_mode", Value::from(mode)),
        ("tachyon_status", Value::from(tachyon_status)),
        (
            "tachyon_search_complete",
            Value::from(proof.tachyon_complete),
        ),
        ("is_tachyon_free", json!(proof.tachyon_free())),
        (
            "min_negative_mass",
            proof.minimum_mass.as_ref().map_or(Value::Null, rational),
        ),
        (
            "min_negative_mass_is_observed_only",
            Value::from(proof.minimum_mass.is_some() && !proof.tachyon_complete),
        ),
        (
            "mass_normalization",
            Value::from("right_mass_shell_q_squared_over_2_plus_N_R_plus_a_R"),
        ),
        ("preserved_supercharge_count", json!(proof.supercharges)),
        (
            "anomaly_status",
            Value::from(if proof.anomalies.is_some() {
                "computed"
            } else if !proof.anomalies_requested {
                "not_requested"
            } else if proof.stage == "anomalies" {
                "incomplete"
            } else if proof.stage == "tachyon_projection" && proof.minimum_mass.is_some() {
                "skipped"
            } else {
                "not_reached"
            }),
        ),
        ("is_anomaly_free", json!(proof.anomalies)),
        (
            "anomaly_checks",
            anomaly.map_or(Value::Null, anomaly_checks),
        ),
    ])
}
/// A bounded-size scan projection; never serializes weights, states or fields.
/// Counts use the spectrum's explicit CPT convention, as do the detailed APIs.
pub fn summary(r: &ModelResult) -> Value {
    let s = r.spectrum();
    let charged_singlets = s
        .fields()
        .iter()
        .filter(|f| {
            f.representations()
                .iter()
                .all(|rep| rep.dimension() == &1.into())
                && f.u1_charges().iter().any(|q| !q.is_zero())
        })
        .count();
    let fi = match r.anomalies() {
        Some(a) => {
            let report = a.report();
            let mut value = status(report.fi().supersymmetric_coefficient(), rational);
            value["has_anomalous_u1"] = json!(report.rotation().anomalous_index().is_some());
            value
        }
        None => json!({"status":"not_requested", "has_anomalous_u1":null}),
    };
    json!({
        "label":r.solved().model().label(),
        "theory":match s.theory() {orbifolder_core::geometry::TheorySpec::Susy=>"susy",orbifolder_core::geometry::TheorySpec::NonSusy=>"non_susy"},
        "anomaly_status":if r.anomalies().is_some(){"computed"}else{"not_requested"},
        "is_anomaly_free":r.anomalies().map(|a|a.report().is_anomaly_free()),
        "is_tachyon_free":r.provenance().tachyon_free(),
        "massless_count":s.fields().len(),
        "tachyon_count":s.tachyonic().fields().len(),
        "gauge_factors":s.basis().root_system().factors().iter().map(|f|f.algebra().to_string()).collect::<Vec<_>>(),
        "u1_count":s.basis().u1_generators().len(),
        "charged_singlet_count":charged_singlets,
        "fi":fi,
        "provenance":provenance(r.provenance(), r.anomalies().map(AnomalyAnalysis::report)),
    })
}
/// Canonical snapshot has sorted map keys and exact integer tokens; no times/addresses.
pub fn canonical_result(r: &Result<ModelResult, ComputeError>) -> String {
    match r {
        Ok(r) => result(r),
        Err(e) => error(e),
    }
    .to_string()
}
