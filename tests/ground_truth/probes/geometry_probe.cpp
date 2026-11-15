// Geometry/group-only native probe: never constructs COrbifold or any states.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include "corbifoldgroup.h"
unsigned SELFDUALLATTICE=1;
#ifdef NON_SUSY
std::vector<int> current_folder_global;
#endif
template<class T> void emit(std::ostream&,const std::vector<T>&);
template<class T> void emit(std::ostream& out,const T& x) { out << x; }
template<class T> void emit(std::ostream& out,const boost::rational<T>& x) { out << "{\"num\":" << x.numerator() << ",\"den\":" << x.denominator() << '}'; }
template<class T> void emit(std::ostream& out,const std::vector<T>& values) {
  out << '['; for(size_t i=0;i<values.size();++i) {if(i) out<<',';emit(out,values[i]);}out<<']';
}
void element(std::ostream& out,const CSpaceGroupElement& g) {
  out << "{\"factors\":[";
#ifdef NON_SUSY
  out << g.Get_m() << ',' << g.Get_n() << ',' << g.Get_k();
#else
  out << g.Get_k() << ',' << g.Get_l() << ",0";
#endif
  out << "],\"translation\":";emit(out,g.GetLatticeElement().Vector());out << '}';
}
int main(int argc,char** argv) {
  if(argc!=3) return 2;
  std::cout.rdbuf(std::cerr.rdbuf());
  std::ifstream input(argv[1]);std::string program;COrbifoldGroup group;
  const bool loaded=group.LoadOrbifoldGroup(input,program);
  std::ofstream out(argv[2]);out<<std::setprecision(std::numeric_limits<double>::max_digits10);
  out << "{\"loaded\":" << (loaded?"true":"false");
  if(loaded) {
    const auto& g=group.GetSpaceGroup();
    out << ",\"modular_prefactors\":";emit(out,g.ShiftsWL_ScalarProductFactors);
    out << ",\"fixed_loci\":[";
    for(size_t i=0;i<group.GetElements().size();++i) {
      if(i) out<<',';
      FPCoordinates fp;const bool exists=g.SG_FixedPoint(group.GetElement(i).SGElement,fp);
      out << "{\"exists\":" << (exists?"true":"false") << ",\"fixed_planes\":[";
      for(size_t j=0;j<3;++j) { if(j)out<<',';out << (fp.FixedTorus[j]?"true":"false"); }
      out << "],\"cartesian\":[";
      for(size_t j=0;j<3;++j) {if(j)out<<',';out<<'['<<fp.Coordinates[j].real()<<','<<fp.Coordinates[j].imag()<<']';}
      out << "]}";
    }
    out << "],\"products\":[";
    const auto& generators=g.GetSG_Generators_Twist();bool first=true;
    for(const auto& a:generators) for(const auto& b:g.GetSG_Generators_Shift()) {
      CSpaceGroupElement product;
      if(!g.SG_Multiply(a,b,product)) return 3;
      if(!first)out<<',';first=false;
      out << "{\"left\":";element(out,a);out << ",\"right\":";element(out,b);out << ",\"result\":";element(out,product);out << '}';
    }
    out << ']';
  }
  out << "}\n";return out?0:4;
}
