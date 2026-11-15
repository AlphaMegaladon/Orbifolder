// Native low-order coupling diagnostics; no original source is patched.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <set>
#include <sstream>
#include "corbifold.h"
#include "cmonomial.h"
unsigned SELFDUALLATTICE=1;
template<class T> void json(std::ostream&,const std::vector<T>&);
void json(std::ostream&,const CVector&);
template<class T> void json(std::ostream& out,const T& x){out<<x;}
template<class T> void json(std::ostream& out,const boost::rational<T>& x){out<<"{\"num\":"<<x.numerator()<<",\"den\":"<<x.denominator()<<'}';}
template<class T> void json(std::ostream& out,const std::vector<T>& v){out<<'[';for(size_t i=0;i<v.size();++i){if(i)out<<',';json(out,v[i]);}out<<']';}
void json(std::ostream& out,const CVector& v){json(out,static_cast<const std::vector<double>&>(v));}
void boolean(std::ostream& out,bool v){out<<(v?"true":"false");}
void rules(std::ostream& out,const std::vector<SDiscreteSymmetry>& r){
  out<<'[';for(size_t i=0;i<r.size();++i){if(i)out<<',';out<<"{\"order\":"<<r[i].Order<<",\"target\":";json(out,r[i].SuperpotentialCharge);out<<",\"operator\":";json(out,r[i].ChargeOperator);out<<'}';}out<<']';
}
bool pre(COrbifold& model,SConfig& config,const std::vector<unsigned>& ids){
  std::ostringstream stream;model.YukawaCouplings.tmp_file=&stream;
  std::vector<std::vector<unsigned>> pools;std::vector<unsigned> positions;
  for(size_t i=0;i<ids.size();++i){pools.push_back({ids[i]});positions.push_back(i);}
  model.YukawaCouplings.CreatePreCouplings(model,config,std::vector<unsigned>(ids.size(),0),positions,pools,0,std::vector<unsigned>(ids.size(),1));
  model.YukawaCouplings.tmp_file=nullptr;return !stream.str().empty();
}
unsigned add(COrbifold& model,SConfig& config,const std::vector<unsigned>& ids){
  config.FieldCouplings.clear();std::vector<std::string> labels;
  for(unsigned i:ids){const auto& f=config.Fields[i];labels.push_back(f.Labels[config.use_Labels]+"_"+std::to_string(f.Numbers[config.use_Labels]));}
  return model.YukawaCouplings.AddCoupling(model,config,labels);
}
int main(int argc,char** argv){
  if(argc!=4)return 2;
  std::cout.rdbuf(std::cerr.rdbuf());
  std::ifstream input(argv[1]);std::string program;COrbifoldGroup group;
  if(!group.LoadOrbifoldGroup(input,program))return 3;
  COrbifold model(group);if(model.GetCheckStatus()!=CheckedAndGood)return 4;
  SConfig config=model.StandardConfig;
  if(!model.YukawaCouplings.Initiate(model.OrbifoldGroup.GetSpaceGroup(),config))return 5;
  // The owned object is non-const. AccessSpaceGroup would invalidate the
  // unrelated legacy CheckStatus graph; parameterize only these rule inputs.
  auto& space=const_cast<CSpaceGroup&>(model.OrbifoldGroup.GetSpaceGroup());
  const auto original_r=space.DiscreteRSymmetries;
  const auto original_non_r=space.DiscreteNonRSymmetries;
  std::vector<SDiscreteSymmetry> explicit_r(3);
  const bool z6=std::string(argv[3]).find("z6")!=std::string::npos;
  for(size_t i=0;i<3;++i){explicit_r[i].Order=z6?(i==0?6:i==1?3:2):3;explicit_r[i].SuperpotentialCharge=1;explicit_r[i].ChargeOperator.assign(4,0);explicit_r[i].ChargeOperator[i+1]=-1;}
  std::vector<unsigned> pool;std::vector<CVector> charges;
  for(size_t i=0;i<config.Fields.size()&&pool.size()<6;++i){const auto& f=config.Fields[i];if(f.Multiplet==LeftChiral&&std::find(charges.begin(),charges.end(),f.q_sh)==charges.end()){pool.push_back(i);charges.push_back(f.q_sh);}}
  for(size_t i=0;i<config.Fields.size()&&pool.size()<6;++i)if(config.Fields[i].Multiplet==LeftChiral&&std::find(pool.begin(),pool.end(),i)==pool.end())pool.push_back(i);
  std::vector<std::vector<unsigned>> trials;
  for(size_t i=0;i<pool.size();++i)for(size_t j=i;j<pool.size();++j)for(size_t k=j;k<pool.size();++k)trials.push_back({pool[i],pool[j],pool[k]});
  std::vector<unsigned> localized;
  for(size_t i=0;i<config.Fields.size()&&localized.size()<3;++i){const auto& f=config.Fields[i];if(f.Multiplet!=LeftChiral||f.GetInternalIndex()[0]==0)continue;bool e6=false;for(const auto& d:f.Dimensions)if(std::abs(d.Dimension)==27)e6=true;if(!e6)continue;
    bool new_brane=true;for(unsigned j:localized)if(config.Fields[j].GetInternalIndex()[0]!=f.GetInternalIndex()[0]||config.Fields[j].GetInternalIndex()[1]==f.GetInternalIndex()[1])new_brane=false;if(new_brane)localized.push_back(i);
  }
  if(localized.size()>=2){trials.push_back({localized[0],localized[0],localized[1]});trials.push_back({localized[0],localized[1],localized[1]});}
  if(localized.size()==3)trials.push_back(localized);
  for(unsigned i:pool){trials.push_back({i});trials.push_back({i,i});}
  // A complete repeated product representation: epsilon x epsilon is symmetric.
  for(size_t i=0;i<config.Fields.size();++i){const auto& f=config.Fields[i];if(f.Multiplet!=LeftChiral)continue;unsigned doublets=0;bool other=false;
    for(const auto& d:f.Dimensions){if(std::abs(d.Dimension)==2)++doublets;else if(std::abs(d.Dimension)!=1)other=true;}
    if(doublets==2&&!other){trials.push_back({unsigned(i),unsigned(i)});break;}
  }
  std::ofstream out(argv[2]);out<<std::setprecision(std::numeric_limits<double>::max_digits10);
  out<<"{\"field_pool\":";json(out,pool);out<<",\"original_r\":";rules(out,original_r);out<<",\"original_non_r\":";rules(out,original_non_r);out<<",\"explicit_r\":";rules(out,explicit_r);out<<",\"trials\":[";
  for(size_t ti=0;ti<trials.size();++ti){if(ti)out<<',';const auto& ids=trials[ti];
    CMonomial monomial(config.SymmetryGroup.GaugeGroup.u1directions.size());
    for(unsigned i:ids){CMonomial atom(config.Fields[i].U1Charges.size());atom.U1Charges=config.Fields[i].U1Charges;atom.GaugeEquivalentFields.push_back({i});atom.Exponents.push_back(1);monomial*=atom;}
    out<<"{\"fields\":";json(out,ids);out<<",\"exponents\":";json(out,monomial.Exponents);out<<",\"monomial_fields\":";json(out,monomial.GaugeEquivalentFields);out<<",\"charge_sum\":";json(out,monomial.U1Charges);
    out<<",\"monomial_allowed\":";boolean(out,monomial.CheckGaugeInvariance(model,config,true,true));out<<",\"non_abelian\":";boolean(out,model.YukawaCouplings.GI.CheckInvariance(model,config,ids));
    out<<",\"original_pre\":";boolean(out,pre(model,config,ids));out<<",\"original_add\":"<<add(model,config,ids);
    space.DiscreteRSymmetries=explicit_r;
    out<<",\"r_charges\":[";for(size_t ri=0;ri<explicit_r.size();++ri){if(ri)out<<',';std::vector<boost::rational<int>> q;for(unsigned id:ids)q.push_back(config.Fields[id].GetDiscreteCharge(explicit_r[ri]));json(out,q);}out<<']';
    out<<",\"explicit_r_pre\":";boolean(out,pre(model,config,ids));out<<",\"explicit_r_add\":"<<add(model,config,ids);
    SConfig singlets=config;for(auto& f:singlets.Fields){f.U1Charges.SetToZero();for(auto& hw:f.HighestWeights_DL)std::fill(hw.begin(),hw.end(),0);for(auto& d:f.Dimensions)d.Dimension=1;}
    space.DiscreteRSymmetries.clear();space.DiscreteNonRSymmetries.clear();
    out<<",\"space_only_add\":"<<add(model,singlets,ids);
    space.DiscreteRSymmetries=original_r;space.DiscreteNonRSymmetries=original_non_r;out<<'}';
  }
  out<<"],\"fi_charge_tests\":[";
  for(int q=-1;q<=1;++q){if(q!=-1)out<<',';CMonomial m(1);m.U1Charges[0]=q;SConfig c=config;c.SymmetryGroup.D0_FI_term=1;
    out<<"{\"charge\":"<<q<<",\"fi\":1,\"demand\":";boolean(out,m.CheckGaugeInvariance(model,c,true,false));out<<",\"relaxed\":";boolean(out,m.CheckGaugeInvariance(model,c,false,false));out<<'}';
  }
  out<<"]}\n";return out?0:6;
}
