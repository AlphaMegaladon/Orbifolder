// Additional native mathematical observations; original model fixtures stay frozen.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <stdexcept>
#include "corbifold.h"
#include "clatticevector.h"

unsigned SELFDUALLATTICE = 1;
#ifdef NON_SUSY
std::vector<int> current_folder_global;
#endif

template<class T> void write(std::ostream& out, const std::vector<T>& values);
void write(std::ostream& out, double x) { out << std::setprecision(std::numeric_limits<double>::max_digits10) << x; }
void write(std::ostream& out, int x) { out << x; }
template<class T> void write(std::ostream& out, const std::vector<T>& values) {
  out << '[';
  for (size_t i=0; i<values.size(); ++i) { if(i) out << ','; write(out,values[i]); }
  out << ']';
}
void write(std::ostream& out, const CVector& v) { write(out,static_cast<const std::vector<double>&>(v)); }
template<class T> void fraction(std::ostream& out, const boost::rational<T>& x) {
  out << "{\"num\":" << x.numerator() << ",\"den\":" << x.denominator() << '}';
}

int main(int argc, char** argv) {
  if(argc!=3) return 2;
  std::cout.rdbuf(std::cerr.rdbuf());
  try {
    std::ifstream input(argv[1]); std::string program;
    COrbifoldGroup group;
    if(!group.LoadOrbifoldGroup(input,program) || group.GetOrbifoldGroup_CheckStatus()!=CheckedAndGood)
      throw std::runtime_error("Invalid model");
    COrbifold model(group);
    if(model.GetCheckStatus()!=CheckedAndGood) throw std::runtime_error("Invalid orbifold");
    S_OscillatorExcitation x; x.NumberOperator=0; x.ZeroPointEnergy=2;
    CMasslessHalfState e8(LeftMover,x), spin(LeftMover,x);
    if(!e8.SolveMassEquation(CVector(16),E8xE8) || !spin.SolveMassEquation(CVector(16),Spin32))
      throw std::runtime_error("Root solver failed");
    std::ofstream out(argv[2]);
    out << "{\"primitives\":{\"spin32_roots\":"; write(out,spin.Weights);
    out << ",\"lattice_cases\":[";
    for(unsigned i=0;i<8;++i) {
      CLatticeVector v(16);
      if(i==1) v[0]=1;
      if(i==2) for(unsigned j=0;j<8;++j) v[j]=0.5;
      if(i==3 || i==4) for(unsigned j=0;j<16;++j) v[j]=0.5;
      if(i==4) v[0]=-0.5;
      if(i==5) { v[0]=1; v[8]=1; }
      if(i==6) v[0]=1.0/3;
      if(i==7) { v[0]=-1; v[1]=1; }
      if(i) out << ',';
      out << "{\"weight\":"; write(out,v);
      out << ",\"e8xe8\":" << (v.From_E8_Lattice(1)&&v.From_E8_Lattice(2)?"true":"false");
      out << ",\"spin32\":" << (v.From_Spin32_Lattice()?"true":"false") << '}';
    }
    out << "],\"so8_cases\":[";
    const double samples[7][4]={{0,0,0,0},{1,0,0,0},{.5,.5,.5,.5},{-.5,.5,.5,.5},{-1,0,0,0},{-1,1,0,0},{.25,.25,.25,.25}};
    for(unsigned i=0;i<7;++i) {
      if(i) out << ',';
      CLatticeVector v(4); for(unsigned j=0;j<4;++j) v[j]=samples[i][j];
      out << "{\"weight\":"; write(out,v);
#ifdef NON_SUSY
      out << ",\"s\":" << int(v.From_SO8S_Lattice()) << ",\"c\":" << int(v.From_SO8C_Lattice());
#else
      out << ",\"ns_r\":" << int(v.From_SO8_Lattice());
#endif
      out << '}';
    }
    CHugeInt large(1); for(unsigned i=0;i<160;++i) large=large*CHugeInt(2);
    boost::rational<CHugeInt> a(large,CHugeInt(3)), b(CHugeInt(1),CHugeInt(7));
    out << "],\"large_arithmetic\":{\"a\":"; fraction(out,a);
    out << ",\"b\":"; fraction(out,b); out << ",\"sum\":"; fraction(out,a+b);
    out << ",\"product\":"; fraction(out,a*b); out << "}},\"factors\":[";
    const auto& factors=model.StandardConfig.SymmetryGroup.GaugeGroup.factor;
    for(size_t i=0;i<factors.size();++i) {
      if(i) out << ',';
      out << "{\"algebra\":\"" << factors[i].algebra << "\",\"cartan\":";
      write(out,calculateCartanMatrix(factors[i].simpleroots));
      out << ",\"parent_root_dynkin\":[";
      for(size_t j=0;j<e8.Weights.size();++j) {
        if(j) out << ',';
        write(out,findDynkinLabels(factors[i],e8.Weights[j]));
      }
      out << "]}";
    }
    out << "]}\n";
    if(!out) throw std::runtime_error("Write failed");
  } catch(const std::exception& e) { std::cerr << e.what() << '\n'; return 1; }
  return 0;
}
