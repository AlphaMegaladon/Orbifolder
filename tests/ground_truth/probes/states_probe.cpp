// Supplemental right-sector oracle. No spectrum/anomaly routines and no Excited reads.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include "csector.h"
unsigned SELFDUALLATTICE=1;
std::vector<int> current_folder_global;
template<class T> void emit(std::ostream&,const std::vector<T>&);
void emit(std::ostream&,const CVector&);
template<class T> void emit(std::ostream& out,const T& x) {out<<x;}
template<class T> void emit(std::ostream& out,const std::vector<T>& xs) {
  out<<'[';for(size_t i=0;i<xs.size();++i){if(i)out<<',';emit(out,xs[i]);}out<<']';
}
void emit(std::ostream& out,const CVector& v) {emit(out,static_cast<const std::vector<double>&>(v));}
void excitation(std::ostream& out,const S_OscillatorExcitation& e) {
  out<<"{\"number_operator\":"<<e.NumberOperator<<",\"squared_weight_target\":"<<e.ZeroPointEnergy<<",\"oscillators\":";
  emit(out,e.OscillatorIndices);out<<'}';
}
void solution(std::ostream& out,const CMasslessHalfState& s) {
  out<<"{\"excitation\":";excitation(out,s.Excitation);out<<",\"weights\":";emit(out,s.Weights);out<<'}';
}
void oscillator(std::ostream& out,const CModedOscillator& o) {
  out<<"{\"direction\":"<<o.GetIndex()<<",\"complex_conjugate\":"<<(o.GetComplex()?"true":"false")
     <<",\"frequency\":"<<o.GetFrequency()<<",\"number_operator\":"<<o.GetNumberOperator()<<'}';
}
void half(std::ostream& out,const CHalfState& h) {
  out<<"{\"solution_index\":"<<h.GetIndex()<<",\"weight_indices\":";emit(out,h.Weights);
  out<<",\"eigenphases\":";emit(out,h.Eigenvalues);out<<'}';
}
template<class T,class F> void emit_array(std::ostream& out,const std::vector<T>& xs,F f) {
  out<<'[';for(size_t i=0;i<xs.size();++i){if(i)out<<',';f(out,xs[i]);}out<<']';
}
int main(int argc,char** argv) {
  if(argc!=2)return 2;
  std::cout.rdbuf(std::cerr.rdbuf());
  std::ofstream out(argv[1]);out<<std::setprecision(std::numeric_limits<double>::max_digits10);
  out<<"{\"cases\":[";
  for(unsigned test=0;test<2;++test) {
    if(test)out<<',';
    std::vector<CTwistVector> twists(3);
    CVector v0(4),v1(4);v0[1]=test?25:1;v0[2]=v0[3]=1;
    v1[1]=1.0/6;v1[2]=-1.0/6;
    twists[0]=v0;twists[1]=v1;
    CTwistVector local;local=v0+v1;
    CSector sector;if(!sector.Create(1,1,0,local,twists))return 3;
    out<<"{\"twist\":";emit(out,static_cast<const CVector&>(local));
    out<<",\"a_left\":"<<local.Get_a_L()<<",\"a_right\":"<<local.Get_a_R();
    out<<",\"left_oscillators\":";emit_array(out,sector.GetLM_all_Oscillators(),oscillator);
    out<<",\"right_oscillators\":";emit_array(out,sector.GetRM_all_Oscillators(),oscillator);
    out<<",\"left_excitations\":";emit_array(out,sector.GetLM_Excitations(),excitation);
    out<<",\"right_excitations\":";emit_array(out,sector.GetRM_Excitations(),excitation);
    out<<",\"right_solutions\":";emit_array(out,sector.GetMasslessRightMovers(),solution);
    out<<",\"right_movers\":";emit_array(out,sector.GetRightMovers(),half);
    out<<",\"tachyonic_right_solutions\":[";
    const auto& tachyons=sector.GetRTachyons();
    for(size_t i=0;i<tachyons.size();++i) {
      if(i)out<<',';
      out<<"{\"solution\":{\"excitation\":";excitation(out,tachyons[i].Excitation);
      out<<",\"weights\":";emit(out,tachyons[i].Weights);out<<"},\"masses\":";emit(out,tachyons[i].tachyonmass);out<<'}';
    }
    out<<"],\"tachyonic_right_movers\":";emit_array(out,sector.TachyonicGetRightMovers(),half);out<<'}';
  }
  out<<"]}\n";return out?0:4;
}
