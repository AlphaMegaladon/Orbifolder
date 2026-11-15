// One serializer, compiled against each original variant. No physics patches.
#include <cmath>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <regex>
#include <sstream>
#include <stdexcept>
#include <vector>
#include "corbifold.h"
#include "cprint.h"
#include "clatticevector.h"

unsigned SELFDUALLATTICE = 1; // CLI globals otherwise defined by excluded main.o.
#ifdef NON_SUSY
std::vector<int> current_folder_global;
#endif

namespace {
struct Capture {
  std::ostream& stream;
  std::streambuf* previous;
  Capture(std::ostream& target, std::streambuf* buffer)
    : stream(target), previous(target.rdbuf(buffer)) {}
  ~Capture() { stream.rdbuf(previous); }
};
void json(std::ostream& out, const std::string& value) {
  out << '"';
  for (unsigned char c : value) {
    switch (c) {
      case '"': out << "\\\""; break;
      case '\\': out << "\\\\"; break;
      case '\n': out << "\\n"; break;
      case '\r': out << "\\r"; break;
      case '\t': out << "\\t"; break;
      default:
        if (c < 32) out << "\\u00" << "0123456789abcdef"[c >> 4]
                        << "0123456789abcdef"[c & 15];
        else out << c;
    }
  }
  out << '"';
}
void json(std::ostream& out, double x) {
  if (!std::isfinite(x)) throw std::runtime_error("Non-finite legacy value");
  out << std::setprecision(std::numeric_limits<double>::max_digits10) << x;
}
void json(std::ostream& out, bool x) { out << (x ? "true" : "false"); }
void json(std::ostream& out, unsigned x) { out << x; }
void json(std::ostream& out, int x) { out << x; }
template<class T> void json(std::ostream& out, const boost::rational<T>& x) {
  out << "{\"num\":" << x.numerator() << ",\"den\":" << x.denominator() << '}';
}
void json(std::ostream&, const CVector&);
void json(std::ostream&, const CSpaceGroupElement&);
template<class T> void json(std::ostream& out, const std::vector<T>& values) {
  out << '[';
  for (size_t i = 0; i < values.size(); ++i) {
    if (i) out << ',';
    json(out, values[i]);
  }
  out << ']';
}
void json(std::ostream& out, const CVector& v) {
  json(out, static_cast<const std::vector<double>&>(v));
}
template<class T> void member(std::ostream& out, const char* key, const T& value) {
  out << ','; json(out, std::string(key)); out << ':'; json(out, value);
}
void json(std::ostream& out, const CSpaceGroupElement& g) {
  out << "{\"factors\":[";
#ifdef NON_SUSY
  out << g.Get_m() << ',' << g.Get_n() << ',' << g.Get_k();
#else
  out << g.Get_k() << ',' << g.Get_l() << ",0";
#endif
  out << ']'; member(out, "translation", g.GetLatticeElement().Vector()); out << '}';
}
void excitation(std::ostream& out, const S_OscillatorExcitation& x) {
  out << "{\"number_operator\":"; json(out, x.NumberOperator);
  member(out, "squared_weight_target", x.ZeroPointEnergy);
  member(out, "oscillators", x.OscillatorIndices); out << '}';
}
template<class Half> void solution(std::ostream& out, const Half& h) {
  out << "{\"excitation\":"; excitation(out, h.Excitation);
  member(out, "weights", h.Weights); out << '}';
}
void half(std::ostream& out, const CHalfState& h) {
  out << "{\"solution_index\":" << h.GetIndex();
  member(out, "weight_indices", h.Weights); member(out, "eigenphases", h.Eigenvalues);
  // The parameterized legacy constructor leaves Excited uninitialized.
  // Export oscillator excitations and actual state containers instead.
  out << '}';
}
void oscillators(std::ostream& out, const std::vector<CModedOscillator>& values) {
  out << '[';
  for (size_t i = 0; i < values.size(); ++i) {
    if (i) out << ',';
    const auto& x = values[i];
    out << "{\"direction\":" << x.GetIndex();
    member(out, "complex_conjugate", x.GetComplex());
    member(out, "frequency", x.GetFrequency());
    member(out, "number_operator", x.GetNumberOperator()); out << '}';
  }
  out << ']';
}
void state(std::ostream& out, const CState& s, const CFixedBrane& b, int kind) {
  out << "{\"left\":"; half(out, s.GetLeftMover());
  out << ",\"right\":"; half(out, s.GetRightMover());
  out << ",\"left_solution\":";
#ifdef NON_SUSY
  if (kind == 1) solution(out, b.TachyonicGetMassLeftMover(s.GetLeftMover().GetIndex()));
  else if (kind == 2) solution(out, b.ExcitedTachyonicGetMassLeftMover(s.GetLeftMover().GetIndex()));
  else
#endif
    solution(out, b.GetMasslessLeftMover(s.GetLeftMover().GetIndex()));
  member(out, "q_charges", s.GetqCharges());
  member(out, "oscillator_contribution", s.GetOsciContribution());
  std::vector<double> phases;
  for (size_t i = 0; i < s.GetNumberOfGammaPhases(); ++i) phases.push_back(s.GetGammaPhase(i));
  member(out, "gamma_phases", phases);
  std::vector<unsigned> fields;
  for (size_t i = 0; i < s.GetNumberOfFieldIndices(); ++i) fields.push_back(s.GetFieldIndex(i));
  member(out, "field_indices", fields); out << '}';
}
void sectors(std::ostream& out, const COrbifold& model) {
  out << '[';
  for (size_t si = 0; si < model.GetNumberOfSectors(); ++si) {
    if (si) out << ',';
    const auto& s = model.GetSector(si);
    out << "{\"index\":" << si << ",\"factors\":[";
#ifdef NON_SUSY
    out << s.Get_m() << ',' << s.Get_n() << ',' << s.Get_k();
#else
    out << s.Get_k() << ',' << s.Get_l() << ",0";
#endif
    out << ']'; member(out, "twist", s.GetTwist());
    member(out, "a_left", s.GetTwist().Get_a_L()); member(out, "a_right", s.GetTwist().Get_a_R());
    out << ",\"left_oscillators\":"; oscillators(out, s.GetLM_all_Oscillators());
    out << ",\"right_oscillators\":"; oscillators(out, s.GetRM_all_Oscillators());
    out << ",\"right_solutions\":[";
    for (size_t i = 0; i < s.GetNumberOfMasslessRightMovers(); ++i) {
      if (i) out << ','; solution(out, s.GetMasslessRightMover(i));
    }
    out << "],\"right_movers\":[";
    for (size_t i = 0; i < s.GetNumberOfRightMovers(); ++i) {
      if (i) out << ','; half(out, s.GetRightMover(i));
    }
    out << "],\"tachyonic_right_solutions\":[";
#ifdef NON_SUSY
    for (size_t i = 0; i < s.GetRTachyons().size(); ++i) {
      if (i) out << ',';
      const auto& h = s.GetRTachyons()[i];
      out << "{\"solution\":"; solution(out, h);
      member(out, "masses", h.tachyonmass); out << '}';
    }
#endif
    out << "],\"tachyonic_right_movers\":[";
#ifdef NON_SUSY
    for (size_t i = 0; i < s.TachyonicGetRightMovers().size(); ++i) {
      if (i) out << ','; half(out, s.TachyonicGetRightMovers()[i]);
    }
#endif
    out << "],\"fixed_branes\":[";
    for (size_t bi = 0; bi < s.GetNumberOfFixedBranes(); ++bi) {
      if (bi) out << ',';
      const auto& b = s.GetFixedBrane(bi);
      out << "{\"index\":" << bi;
      member(out, "constructing_index", b.Getconstructing_Element());
      member(out, "element", b.GetSGElement()); member(out, "label", b.GetFixedBraneLabel());
      out << ",\"left_solutions\":[";
      for (size_t i = 0; i < b.GetNumberOfMasslessLeftMovers(); ++i) {
        if (i) out << ','; solution(out, b.GetMasslessLeftMover(i));
      }
      out << "],\"left_movers\":[";
      for (size_t i = 0; i < b.GetNumberOfLeftMovers(); ++i) {
        if (i) out << ','; half(out, b.GetLeftMover(i));
      }
      out << "],\"states\":[";
      for (size_t i = 0; i < b.GetNumberOfInvariantStates(); ++i) {
        if (i) out << ','; state(out, b.GetInvariantState(i), b, 0);
      }
      out << "],\"tachyonic_states\":[";
#ifdef NON_SUSY
      for (size_t i = 0; i < b.TachyonicGetNumberOfInvariantStates(); ++i) {
        if (i) out << ','; state(out, b.TachyonicGetInvariantState(i), b, 1);
      }
#endif
      out << "],\"excited_tachyonic_states\":[";
#ifdef NON_SUSY
      for (size_t i = 0; i < b.ExcitedTachyonicGetNumberOfInvariantStates(); ++i) {
        if (i) out << ','; state(out, b.ExcitedTachyonicGetInvariantState(i), b, 2);
      }
#endif
      out << "]}";
    }
    out << "]}";
  }
  out << ']';
}
void spectrum(std::ostream& out, const SConfig& config, const CSpaceGroup& geometry) {
  out << "{\"field_count\":" << config.Fields.size() << ",\"fields\":[";
  for (size_t i = 0; i < config.Fields.size(); ++i) {
    if (i) out << ',';
    const auto& f = config.Fields[i];
    out << "{\"index\":" << i << ",\"multiplet\":" << int(f.Multiplet);
    member(out, "source_index", f.GetInternalIndex()); member(out, "element", f.SGElement);
    member(out, "weight_indices", f.WeightIndices); member(out, "highest_weights", f.HighestWeights_DL);
    member(out, "u1_charges", f.U1Charges);
    // non-SUSY CreateRepresentations disables the assignment of CField::q_sh.
    // The native right-moving weights remain available in the linked state.
    out << ",\"q_sh\":";
    if (f.q_sh.empty()) out << "null"; else json(out, f.q_sh);
    member(out, "oscillator_contribution", f.OsciContribution); member(out, "gamma_phases", f.gamma_phases);
    member(out, "labels", f.Labels); member(out, "numbers", f.Numbers);
    out << ",\"representations\":[";
    for (size_t j = 0; j < f.Dimensions.size(); ++j) {
      if (j) out << ',';
      out << "{\"dimension\":" << f.Dimensions[j].Dimension;
      member(out, "label", f.Dimensions[j].AdditionalLabel); out << '}';
    }
    out << "],\"discrete_r_charges\":[";
    for (size_t j = 0; j < geometry.DiscreteRSymmetries.size(); ++j) {
      if (j) out << ','; json(out, f.GetDiscreteCharge(geometry.DiscreteRSymmetries[j]));
    }
    out << "],\"discrete_non_r_charges\":[";
    for (size_t j = 0; j < geometry.DiscreteNonRSymmetries.size(); ++j) {
      if (j) out << ','; json(out, f.GetDiscreteCharge(geometry.DiscreteNonRSymmetries[j]));
    }
    out << "]}";
  }
  out << "]}";
}
void geometry(std::ostream& out, const CSpaceGroup& g) {
  out << "{\"orders\":[" << g.GetM() << ',' << g.GetN() << ',';
#ifdef NON_SUSY
  out << g.GetK();
#else
  out << '1';
#endif
  out << ']'; member(out, "status", int(g.GetCheckStatus())); member(out, "lattice_label", g.lattice_label);
  out << ",\"torus_basis\":[";
  const auto& basis = g.GetT6Lattice();
  for (size_t i = 0; i < basis.size(); ++i) {
    if (i) out << ','; out << '[';
    for (size_t j = 0; j < basis[i].size(); ++j) {
      if (j) out << ','; out << '['; json(out, basis[i][j].real());
      out << ','; json(out, basis[i][j].imag()); out << ']';
    }
    out << ']';
  }
  out << ']'; member(out, "twists", g.GetTwists());
  member(out, "twist_generators", g.GetSG_Generators_Twist());
  member(out, "translation_generators", g.GetSG_Generators_Shift());
  member(out, "constructing_elements", g.GetSectors()); member(out, "centralizers", g.GetCentralizer());
  member(out, "wilson_line_orders", g.GetWL_AllowedOrders()); member(out, "wilson_line_relations", g.GetWL_Relations());
  out << ",\"discrete_symmetries\":[";
  for (unsigned kind = 0; kind < 2; ++kind) {
    if (kind) out << ',';
    const auto& symmetries = kind ? g.DiscreteRSymmetries : g.DiscreteNonRSymmetries;
    out << '[';
    for (size_t j = 0; j < symmetries.size(); ++j) {
      if (j) out << ',';
      const auto& symmetry = symmetries[j];
      out << "{\"order\":" << symmetry.Order;
      member(out, "superpotential_charge", symmetry.SuperpotentialCharge);
      member(out, "charge_operator", symmetry.ChargeOperator);
      member(out, "generator", symmetry.SG_SymmetryGenerator); out << '}';
    }
    out << ']';
  }
  out << ']';
  out << ",\"rotation_actions\":[";
  for (size_t i = 0; i < g.GetSG_Generators_Twist().size(); ++i) {
    if (i) out << ',';
    const auto& rotation = g.GetSG_Generators_Twist()[i];
    std::vector<rationalVector> matrix(6, rationalVector(6));
    for (unsigned col = 0; col < 6; ++col) {
      CSpaceGroupElement translation, product;
      translation.Set_n_alpha(col, boost::rational<int>(1));
      if (!g.SG_Multiply(rotation, translation, product)) throw std::runtime_error("SG_Multiply failed");
      for (unsigned row = 0; row < 6; ++row) matrix[row][col] = product.Get_n(row) - rotation.Get_n(row);
    }
    out << "{\"generator\":"; json(out, rotation); member(out, "matrix", matrix); out << '}';
  }
  out << "]}";
}
void orbifold_element(std::ostream& out, const COrbifoldGroupElement& e) {
  out << "{\"element\":"; json(out, e.SGElement);
  member(out, "shift", e.Shift); member(out, "twist", e.Twist); out << '}';
}
void group(std::ostream& out, const COrbifoldGroup& g) {
  out << "{\"label\":"; json(out, g.Label);
  member(out, "lattice", int(g.GetLattice())); member(out, "supersymmetry", g.GetNumberOfSupersymmetry());
  member(out, "invariant_supercharges", g.GetInvariantSupercharges());
  member(out, "shifts", g.GetShifts());
  member(out, "wilson_lines", g.GetWilsonLines().GetWilsonLines());
  member(out, "use_freely_acting_wilson_line", g.UseFreelyActingWL);
  member(out, "freely_acting_wilson_line", g.FreelyActingWilsonLine);
  member(out, "loaded_u1_generators", g.LoadedU1Generators);
  out << ",\"elements\":[";
  for (size_t i = 0; i < g.GetElements().size(); ++i) {
    if (i) out << ','; orbifold_element(out, g.GetElement(i));
  }
  out << "],\"centralizers\":[";
  for (size_t i = 0; i < g.GetElements().size(); ++i) {
    if (i) out << ','; out << '[';
    for (size_t j = 0; j < g.GetCentralizer(i).size(); ++j) {
      if (j) out << ','; orbifold_element(out, g.GetCentralizer(i)[j]);
    }
    out << ']';
  }
  out << ']';
  member(out, "modular_invariance_status", int(g.GetModularInvariance_CheckStatus()));
  member(out, "status", int(g.GetOrbifoldGroup_CheckStatus()));
  const auto& t = g.GetDiscreteTorsion();
  out << ",\"torsion\":{\"enabled\":"; json(out, t.UseGDT);
  member(out, "a", t.a); member(out, "b", t.b); member(out, "c", t.c); member(out, "d", t.d);
  out << "}}";
}
void gauge(std::ostream& out, const SSymmetryGroup& g) {
  out << "{\"factors\":[";
  for (size_t i = 0; i < g.GaugeGroup.factor.size(); ++i) {
    if (i) out << ',';
    const auto& f = g.GaugeGroup.factor[i];
    out << "{\"algebra\":"; json(out, f.algebra); member(out, "rank", f.rank);
    member(out, "simple_roots", f.simpleroots); out << '}';
  }
  out << ']'; member(out, "u1_generators", g.GaugeGroup.u1directions);
  member(out, "basis", g.GaugeGroup.basis); member(out, "first_u1_anomalous", g.IsFirstU1Anomalous);
  member(out, "fi_term", g.D0_FI_term); out << '}';
}
void math(std::ostream& out) {
  boost::rational<int> a(1, 3), b(-1, 6);
  out << "{\"rational_arithmetic\":{\"a\":"; json(out, a); member(out, "b", b);
  member(out, "sum", a + b); member(out, "product", a * b); member(out, "quotient", a / b);
  out << "},\"parent_e8xe8_roots\":";
  S_OscillatorExcitation x; x.NumberOperator = 0; x.ZeroPointEnergy = 2;
  CMasslessHalfState roots(LeftMover, x);
  if (!roots.SolveMassEquation(CVector(16), E8xE8)) throw std::runtime_error("Root solver failed");
  json(out, roots.Weights); out << ",\"so8_samples\":[";
  const double samples[3][4] = {{1,0,0,0},{.5,.5,.5,.5},{-.5,.5,.5,.5}};
  for (unsigned i = 0; i < 3; ++i) {
    if (i) out << ',';
    CLatticeVector v(4); for (unsigned j = 0; j < 4; ++j) v[j] = samples[i][j];
    out << "{\"weight\":"; json(out, v);
#ifdef NON_SUSY
    member(out, "so8_s_class", int(v.From_SO8S_Lattice()));
    member(out, "so8_c_class", int(v.From_SO8C_Lattice()));
#else
    member(out, "ns_r_class", int(v.From_SO8_Lattice()));
#endif
    out << '}';
  }
  out << "]}";
}
void anomalies(std::ostream& out, COrbifold& model) {
  SConfig checked = model.StandardConfig;
  std::ostringstream report;
  CPrint print(Tstandard, &report);
  report << std::setprecision(std::numeric_limits<double>::max_digits10);
  CGaugeIndices indices;
  bool passed;
  { // Some original trace statements write to cout instead of Print.out.
    Capture capture(std::cout, report.rdbuf());
    passed = model.CheckAnomaly(checked, indices, print, true);
  }
  out << "{\"check_passed\":"; json(out, passed);
  member(out, "report", report.str());
  out << ",\"gauge_after_check\":"; gauge(out, checked.SymmetryGroup);
  out << ",\"coefficients\":[";
  // These are already computed rational/integer traces from CheckAnomaly,
  // not independently recomputed anomaly sums or low-precision field dumps.
  std::regex pattern("^\\s*(.*?)\\s*=\\s*(-?[0-9]+)(?:/([0-9]+))?\\s*$");
  std::istringstream lines(report.str()); std::string line; bool first = true;
  while (std::getline(lines, line)) {
    std::smatch m;
    if (!std::regex_match(line, m, pattern)) continue;
    std::string label = m[1].str();
    if (label.find("tr ") == std::string::npos && label.find("#(doublets)") == std::string::npos) continue;
    if (!first) out << ','; first = false;
    out << "{\"label\":"; json(out, label);
    out << ",\"value\":{\"num\":" << m[2].str() << ",\"den\":"
        << (m[3].matched ? m[3].str() : "1") << "}}";
  }
  out << "]}";
}
void susy_analysis(std::ostream& out, COrbifold& model) {
#ifdef NON_SUSY
  out << "{\"status\":\"not_applicable\"}";
#else
  SConfig config = model.StandardConfig;
  std::vector<unsigned> pool;
  std::vector<CVector> distinct_charges;
  for (size_t i = 0; i < config.Fields.size() && pool.size() < 6; ++i)
    if (config.Fields[i].Multiplet == LeftChiral &&
        std::find(distinct_charges.begin(), distinct_charges.end(), config.Fields[i].q_sh) == distinct_charges.end()) {
      pool.push_back(i); distinct_charges.push_back(config.Fields[i].q_sh);
    }
  for (size_t i = 0; i < config.Fields.size() && pool.size() < 6; ++i)
    if (config.Fields[i].Multiplet == LeftChiral && std::find(pool.begin(), pool.end(), i) == pool.end()) pool.push_back(i);
  if (pool.empty() || !model.YukawaCouplings.Initiate(model.OrbifoldGroup.GetSpaceGroup(), config))
    throw std::runtime_error("SUSY check could not be initialized");
  out << "{\"status\":\"computed\",\"scope\":\"cubic combinations of up to six left-chiral fields; distinct q_sh first\"";
  member(out, "field_pool", pool); out << ",\"trials\":["; bool first = true;
  for (size_t a = 0; a < pool.size(); ++a) for (size_t b = a; b < pool.size(); ++b)
    for (size_t c = b; c < pool.size(); ++c) {
      if (!first) out << ','; first = false;
      std::vector<unsigned> ids = {pool[a], pool[b], pool[c]};
      std::vector<std::string> labels;
      CMonomial monomial(config.SymmetryGroup.GaugeGroup.u1directions.size());
      for (unsigned id : ids) {
        const auto& f = config.Fields[id];
        labels.push_back(f.Labels.at(config.use_Labels) + "_" + std::to_string(f.Numbers.at(config.use_Labels)));
        CMonomial atom(f.U1Charges.size()); atom.U1Charges = f.U1Charges;
        atom.GaugeEquivalentFields.push_back(std::vector<unsigned>(1, id)); atom.Exponents.push_back(1);
        monomial *= atom;
      }
      bool invariant = monomial.CheckGaugeInvariance(model, config, true, true);
      config.FieldCouplings.clear();
      unsigned count = model.YukawaCouplings.AddCoupling(model, config, labels);
      out << "{\"field_indices\":"; json(out, ids); member(out, "labels", labels);
      member(out, "gauge_invariant", invariant); member(out, "allowed_couplings", count);
      member(out, "monomial_exponents", monomial.Exponents);
      member(out, "monomial_fields", monomial.GaugeEquivalentFields);
      member(out, "u1_charge_sum", monomial.U1Charges); out << '}';
    }
  out << "]}";
#endif
}
} // namespace

int main(int argc, char** argv) {
  if (argc != 3) { std::cerr << "Usage: oracle MODEL OUTPUT.json\n"; return 2; }
  std::cout.rdbuf(std::cerr.rdbuf());
  std::cout << std::setprecision(std::numeric_limits<double>::max_digits10);
  std::srand(0);
  try {
    std::ifstream input(argv[1]); std::string program;
    COrbifoldGroup loaded;
    if (!input || !loaded.LoadOrbifoldGroup(input, program) ||
        loaded.GetOrbifoldGroup_CheckStatus() != CheckedAndGood)
      throw std::runtime_error("Model loading/validation failed");
    COrbifold model(loaded);
    if (model.GetCheckStatus() != CheckedAndGood) throw std::runtime_error("Orbifold construction failed");
    std::ofstream out(argv[2]);
    if (!out) throw std::runtime_error("Cannot create output");
    out << "{\"schema_version\":1,\"variant\":";
#ifdef NON_SUSY
    json(out, std::string("non_susy"));
#else
    json(out, std::string("susy"));
#endif
    out << ",\"conventions\":{\"seed\":0,\"multiplets\":";
    std::vector<std::string> multiplets = {"NOT_DEF_SUSY","AnyKind","LeftChiral","RightChiral", "Vector", "VectorCC", "Hyper", "Halfhyper", "Gravity", "GravityCC", "LCModulus", "RCModulus"};
#ifdef NON_SUSY
    const std::vector<std::string> extra = {"BOSE_mu","Gauge","bGauge","Scalar","bScalar","moduli","bmoduli","LeftFermi","RightFermi","Tachyon","bTachyon","Excited_Tachyon","Excited_bTachyon"};
    multiplets.insert(multiplets.end(), extra.begin(), extra.end());
#endif
    json(out, multiplets);
    member(out, "factor_mapping", std::string("susy (k_old,l_old)->(m,n,0); non_susy (m,n,k) unchanged"));
    member(out, "numbers", std::string("native rationals as integer num/den; legacy doubles at max_digits10, never rationalized by exporter"));
    out << '}';
    out << ",\"math\":"; math(out);
    out << ",\"geometry\":"; geometry(out, model.OrbifoldGroup.GetSpaceGroup());
    out << ",\"orbifold_group\":"; group(out, model.OrbifoldGroup);
    out << ",\"gauge_group\":"; gauge(out, model.StandardConfig.SymmetryGroup);
    out << ",\"sectors\":"; sectors(out, model);
    out << ",\"spectrum\":"; spectrum(out, model.StandardConfig, model.OrbifoldGroup.GetSpaceGroup());
    out << ",\"tachyonic_spectrum\":";
#ifdef NON_SUSY
    spectrum(out, model.TachyonicStandardConfig, model.OrbifoldGroup.GetSpaceGroup());
#else
    out << "{\"field_count\":0,\"fields\":[]}";
#endif
    out << ",\"anomalies\":"; anomalies(out, model);
    out << ",\"susy_analysis\":"; susy_analysis(out, model);
    out << ",\"coverage\":{\"massless_states\":\"all public solutions and invariant states\","
        "\"tachyonic_left_solutions\":\"solutions referenced by invariant states\","
        "\"tachyon_search\":\"original C++ bounds; no claim beyond those bounds\","
        "\"anomaly_coefficients\":\"native rational/integer traces emitted by CheckAnomaly\"}}\n";
    if (!out) throw std::runtime_error("Output write failed");
  } catch (const std::exception& e) { std::cerr << "Oracle failed: " << e.what() << '\n'; return 1; }
}
