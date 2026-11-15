// Observe native index tables and CheckAnomaly under linear charge probes.
// Does not calculate an alternative anomaly sum.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <sstream>
#include <stdexcept>
#include "corbifold.h"
#include "cprint.h"
#include "cgaugeinvariance.h"
unsigned SELFDUALLATTICE=1;
#ifdef NON_SUSY
std::vector<int> current_folder_global;
#endif
void quoted(std::ostream& out,const std::string& s) {
  out<<'"';for(unsigned char c:s) {
    if(c=='"'||c=='\\')out<<'\\'<<c;
    else if(c=='\n')out<<"\\n";else if(c=='\r')out<<"\\r";else if(c=='\t')out<<"\\t";
    else if(c<32)throw std::runtime_error("Unexpected control byte");else out<<c;
  }out<<'"';
}
template<class T> void values(std::ostream& out,const std::vector<T>& v) {
  out<<'[';for(size_t i=0;i<v.size();++i){if(i)out<<',';out<<v[i];}out<<']';
}
bool active(const CField& f) {
#ifdef NON_SUSY
  return f.Multiplet==LeftFermi;
#else
  return f.Multiplet==LeftChiral;
#endif
}
void check(std::ostream& out,COrbifold& model,SConfig config,CGaugeIndices& indices) {
  std::ostringstream report;CPrint print(Tstandard,&report);
  auto* previous=std::cout.rdbuf(report.rdbuf());
  const bool passed=model.CheckAnomaly(config,indices,print,true);
  std::cout.rdbuf(previous);
  out<<"{\"check_passed\":"<<(passed?"true":"false")<<",\"report\":";quoted(out,report.str());out<<'}';
}
int main(int argc,char** argv) {
  if(argc!=3)return 2;
  std::cout.rdbuf(std::cerr.rdbuf());
  try {
    std::ifstream input(argv[1]);std::string program;COrbifoldGroup group;
    if(!group.LoadOrbifoldGroup(input,program))throw std::runtime_error("Load failed");
    COrbifold model(group);if(model.GetCheckStatus()!=CheckedAndGood)throw std::runtime_error("Model failed");
    const SConfig& config=model.StandardConfig;
    const auto& factors=config.SymmetryGroup.GaugeGroup.factor;
    const auto& u1=config.SymmetryGroup.GaugeGroup.u1directions;
    CGaugeIndices indices;
    std::ofstream out(argv[2]);out<<std::setprecision(std::numeric_limits<double>::max_digits10);
    out<<"{\"exceptional_quadratic\":{\"E6_27\":"<<indices.GetQuadraticIndices().at(2).at(6).at(27)
       <<",\"E6_adj\":"<<indices.GetQuadraticIndicesAdj().at(2).at(6)
       <<",\"E7_56\":"<<indices.GetQuadraticIndices().at(2).at(7).at(56)
       <<",\"E7_adj\":"<<indices.GetQuadraticIndicesAdj().at(2).at(7)
       <<",\"E8_adj\":"<<indices.GetQuadraticIndicesAdj().at(2).at(8)<<"},\"fields\":[";bool first=true;
    for(size_t i=0;i<config.Fields.size();++i) {
      const auto& f=config.Fields[i];if(!active(f))continue;
      if(!first)out<<',';first=false;
      out<<"{\"index\":"<<i<<",\"quadratic\":[";
      for(size_t g=0;g<factors.size();++g) {
        if(g)out<<',';unsigned index=0;
        if(!indices.GetQuadraticIndex(factors[g],std::abs(f.Dimensions[g].Dimension),index))throw std::runtime_error("Unknown quadratic index");
        out<<index;
      }
      out<<"],\"cubic\":[";
      for(size_t g=0;g<factors.size();++g) {
        if(g)out<<',';
        if(factors[g].algebra[0]!='A'||factors[g].rank<2){out<<0;continue;}
        const int d=f.Dimensions[g].Dimension;
        const int index=indices.GetCubicIndices().at(factors[g].rank).at(std::abs(d));
        if(index<0)throw std::runtime_error("Unknown cubic index");
        out<<(d<0?-index:index);
      }out<<"]}";
    }
    out<<"],\"polarizations\":[";first=true;
    for(size_t i=0;i<u1.size();++i)for(size_t j=i+1;j<u1.size();++j)for(size_t k=j+1;k<u1.size();++k) {
      SConfig sample=config;sample.SymmetryGroup.IsFirstU1Anomalous=false;sample.SymmetryGroup.D0_FI_term=0;
      std::vector<double> direction(16);for(size_t n=0;n<16;++n)direction[n]=u1[i][n]+u1[j][n]+u1[k][n];
      sample.SymmetryGroup.GaugeGroup.u1directions.assign(1,direction);
      for(auto& f:sample.Fields){const double q=f.U1Charges[i]+f.U1Charges[j]+f.U1Charges[k];f.U1Charges.Assign(1,q);}
      if(!first)out<<',';first=false;
      out<<"{\"indices\":["<<i<<','<<j<<','<<k<<"],\"native\":";check(out,model,sample,indices);out<<'}';
    }
    out<<"],\"removed_chiral\":";
    size_t remove=config.Fields.size();
    for(size_t i=0;i<config.Fields.size()&&remove==config.Fields.size();++i)if(active(config.Fields[i])) {
      for(size_t g=0;g<factors.size();++g)if(factors[g].algebra[0]=='A'&&factors[g].rank>1&&indices.GetCubicIndices().at(factors[g].rank).at(std::abs(config.Fields[i].Dimensions[g].Dimension))>0){remove=i;break;}
    }
    if(remove==config.Fields.size())out<<"null";
    else {SConfig sample=config;sample.Fields.erase(sample.Fields.begin()+remove);out<<"{\"field_index\":"<<remove<<",\"native\":";check(out,model,sample,indices);out<<'}';}
    out<<"}\n";return out?0:4;
  }catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
}
