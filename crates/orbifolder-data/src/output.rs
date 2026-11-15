use orbifolder_core::math_core::Rational;
use orbifolder_core::{
    analysis::Computation,
    anomalies::{AnomalyAnalysis, AnomalyCheck, AnomalyTraces},
    execution::{ComputeError, ModelResult},
    geometry::{LocalElement, SpaceGroupElement},
    math_core::{Matrix, Vector},
    physics_engine::{
        Excitation, Field, HalfState, MassShell, Multiplet, Oscillator, ProjectedState, Spectrum,
    },
    susy::{CouplingReport, RuleCharge, Superpotential},
};
use serde_json::{Number, Value, json};
pub fn rational(r: &Rational) -> Value {
    json!({"num":integer(&r.numerator()),"den":integer(&r.denominator())})
}
pub fn integer(n: &num_bigint::BigInt) -> Value {
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
    let kind = match f.multiplet() {
        Multiplet::Susy(m) => json!({"family":"susy","kind":format!("{m:?}")}),
        Multiplet::Particle(p) => json!({"family":"particle","kind":format!("{p:?}")}),
        Multiplet::Tachyon { excited, conjugate } => {
            json!({"family":"tachyon","excited":excited,"conjugate":conjugate})
        }
    };
    json!({"id":id,"source":f.source(),"element":element(f.element()),"weight_indices":f.weight_indices(),"highest_weight":vector(f.highest_weight()),"representations":f.representations().iter().map(|r|json!({"algebra":r.algebra().to_string(),"dynkin":r.dynkin_labels().iter().map(integer).collect::<Vec<_>>(),"dimension":integer(r.dimension())})).collect::<Vec<_>>(),"u1_charges":ratios(f.u1_charges()),"multiplet":kind,"right_weights":f.right_weights().iter().map(vector).collect::<Vec<_>>(),"origin_classes":f.origin_classes().iter().map(|c|format!("{c:?}")).collect::<Vec<_>>(),"spacetime_classes":f.spacetime_classes().iter().map(|c|format!("{c:?}")).collect::<Vec<_>>(),"scalar_charge":f.scalar_charge().map(vector),"left_excitation":excitation(f.left_excitation()),"right_excitation":excitation(f.right_excitation()),"gamma":ratios(f.gamma()),"oscillator_contribution":vector(f.oscillator_contribution()),"mass":rational(f.mass())})
}
pub fn spectrum(s: &Spectrum) -> Value {
    json!({"theory":format!("{:?}",s.theory()),"gauge_group":{"roots":s.basis().root_system().roots().iter().map(vector).collect::<Vec<_>>(),"factors":s.basis().root_system().factors().iter().map(|f|json!({"algebra":f.algebra().to_string(),"simple_roots":f.simple_roots().iter().map(vector).collect::<Vec<_>>(),"cartan":matrix(&f.cartan_matrix().expect("validated Lie factor"))})).collect::<Vec<_>>(),"u1_generators":s.basis().u1_generators().iter().map(vector).collect::<Vec<_>>()},"fields":s.fields().iter().enumerate().map(|(id,f)|field(id,f)).collect::<Vec<_>>(),"tachyons":s.tachyonic().fields().iter().enumerate().map(|(id,f)|field(id,f)).collect::<Vec<_>>()})
}
fn traces(t: &AnomalyTraces) -> Value {
    let n = t.u1_count();
    json!({"gravitational":ratios(t.gravitational()),"mixed":t.mixed().iter().map(|r|ratios(r)).collect::<Vec<_>>(),"gauge_cubic":ratios(t.gauge_cubic()),"abelian_cubic":(0..n).map(|i|(0..n).map(|j|ratios(&(0..n).map(|k|t.abelian_cubic(i,j,k).unwrap().clone()).collect::<Vec<_>>())).collect::<Vec<_>>()).collect::<Vec<_>>(),"su2_doublets":t.su2_doublets().iter().map(|x|x.as_ref().map(integer)).collect::<Vec<_>>(),"su2_parity":t.su2_parity()})
}
pub fn status<T>(c: &Computation<T>, encode: impl Fn(&T) -> Value) -> Value {
    match c {
        Computation::Computed(v) => json!({"status":"computed","value":encode(v)}),
        Computation::NotApplicable(reason) => json!({"status":"not_applicable","reason":reason}),
        Computation::Unsupported(reason) => json!({"status":"unsupported","reason":reason}),
    }
}
pub fn anomalies(a: &AnomalyAnalysis) -> Value {
    let r = a.report();
    let rotation = r.rotation();
    let fi = r.fi();
    json!({"status":"computed","is_anomaly_free":r.is_anomaly_free(),"raw":traces(r.raw()),"canonical":traces(r.canonical()),"rotation":{"generators":rotation.generators().iter().map(vector).collect::<Vec<_>>(),"transformation":matrix(rotation.transformation()),"gram":matrix(rotation.gram()),"trace_vector":vector(rotation.trace_vector()),"anomalous_index":rotation.anomalous_index()},"levels":ratios(r.non_abelian_levels()),"fi":{"legacy_d0":rational(fi.legacy_d0()),"norm_squared":rational(fi.anomalous_norm_squared()),"trace_over24":rational(fi.gs_trace_over24()),"susy_coefficient":status(fi.supersymmetric_coefficient(),rational)},"checks":r.checks().iter().map(|c|{
        let kind=match c.kind(){AnomalyCheck::GaugeCubic{factor}=>json!({"kind":"gauge_cubic","factor":factor}),AnomalyCheck::Witten{factor}=>json!({"kind":"witten","factor":factor}),AnomalyCheck::MixedUniversal{u1,factor}=>json!({"kind":"mixed_universal","u1":u1,"factor":factor}),AnomalyCheck::AbelianUniversal{indices}=>json!({"kind":"abelian_universal","indices":indices}),AnomalyCheck::TracelessGravity{u1}=>json!({"kind":"traceless_gravity","u1":u1}),AnomalyCheck::TracelessMixed{u1,factor}=>json!({"kind":"traceless_mixed","u1":u1,"factor":factor}),AnomalyCheck::IndependentAbelianDirections=>json!({"kind":"independent_abelian_directions"}),AnomalyCheck::FiNormalization=>json!({"kind":"fi_normalization"})};
        json!({"check":kind,"result":status(c.status(),|b|json!(b)),"actual":c.actual().map(rational),"expected":c.expected().map(rational)})
    }).collect::<Vec<_>>(),"spectrum":spectrum(a.spectrum())})
}
pub fn coupling(c: &CouplingReport) -> Value {
    let i = c.invariant();
    json!({"fields":i.monomial().fields(),"degree":i.monomial().degree(),"charge_sum":ratios(i.charge_sum()),"tensor_singlets":integer(i.tensor_singlets()),"polynomial_singlets":integer(i.polynomial_singlets()),"gauge_invariant":i.gauge_invariant(),"native_gauge_invariant":i.native_gauge_invariant(),"native_pre":c.native_precoupling(),"native_allowed":c.native_allowed(),"polynomial_allowed":c.polynomial_allowed(),"point_group":c.point_group(),"space_group":c.space_group(),"native_fixed_torus":c.native_fixed_torus(),"gamma_sums":ratios(c.gamma_sums()),"gamma":status(c.gamma(),|b|json!(b)),"discrete":c.discrete().iter().map(|check|{let r=check.rule();let (kind,op)=match r.operator(){RuleCharge::R(op)=>("r",ratios(op.as_ref())),RuleCharge::NonR(op)=>("non_r",ratios(op.as_ref()))};json!({"kind":kind,"order":r.order(),"target":rational(r.target()),"operator":op,"charge_sum":rational(check.charge_sum()),"passed":check.passed()})}).collect::<Vec<_>>()})
}
pub fn superpotential(w: &Superpotential) -> Value {
    json!({"status":"computed","max_degree":w.max_degree(),"field_pool":w.field_pool(),"candidates":w.candidates().iter().map(coupling).collect::<Vec<_>>(),"allowed":w.allowed().map(|(id,_)|id).collect::<Vec<_>>()})
}
pub fn result(r: &ModelResult) -> Value {
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
    };
    json!({"status":"error","category":category,"message":e.to_string()})
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
        "is_tachyon_free":s.tachyonic().fields().is_empty(),
        "massless_count":s.fields().len(),
        "tachyon_count":s.tachyonic().fields().len(),
        "gauge_factors":s.basis().root_system().factors().iter().map(|f|f.algebra().to_string()).collect::<Vec<_>>(),
        "u1_count":s.basis().u1_generators().len(),
        "charged_singlet_count":charged_singlets,
        "fi":fi,
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
