// Native Freudenthal characters and tensor/self-tensor outputs, not copied tables.
#include <fstream>
#include <iomanip>
#include <iostream>
#include <map>
#include <limits>
#include "corbifold.h"
#include "cgaugeinvariance.h"
unsigned SELFDUALLATTICE=1;
typedef std::map<std::vector<int>,int> Character;
struct Record{unsigned factor;std::vector<int> highest;int dimension;Character weights;};
template<class T> void json(std::ostream& out,const std::vector<T>& v){out<<'[';for(size_t i=0;i<v.size();++i){if(i)out<<',';out<<v[i];}out<<']';}
void character(std::ostream& out,const Character& c){out<<'[';bool first=true;for(const auto& w:c){if(w.second==0)continue;if(!first)out<<',';first=false;out<<"{\"weight\":";json(out,w.first);out<<",\"multiplicity\":"<<w.second<<'}';}out<<']';}
std::vector<std::vector<int>> expanded(const Character& c){std::vector<std::vector<int>> v;for(const auto& w:c)for(int i=0;i<w.second;++i)v.push_back(w.first);return v;}
void decomposed(std::ostream& out,const std::vector<std::vector<std::vector<int>>>& v){out<<'[';for(size_t i=0;i<v.size();++i){if(i)out<<',';Character c;for(const auto& w:v[i])++c[w];character(out,c);}out<<']';}
size_t count(const std::vector<std::vector<std::vector<int>>>& v){size_t n=0;for(const auto& w:v)n+=w.size();return n;}
Character joined(const std::vector<std::vector<std::vector<int>>>& v){Character c;for(const auto& irrep:v)for(const auto& w:irrep)++c[w];return c;}
int main(int argc,char** argv){
  if(argc!=3)return 2;std::cout.rdbuf(std::cerr.rdbuf());
  std::ifstream input(argv[1]);std::string program;COrbifoldGroup group;
  if(!group.LoadOrbifoldGroup(input,program))return 3;
  COrbifold model(group);if(model.GetCheckStatus()!=CheckedAndGood)return 4;
  const auto& factors=model.StandardConfig.SymmetryGroup.GaugeGroup.factor;
  std::vector<Record> records;std::map<std::pair<unsigned,std::vector<int>>,size_t> seen;
  for(const auto& field:model.StandardConfig.Fields)for(unsigned g=0;g<factors.size();++g){
    const auto key=std::make_pair(g,field.HighestWeights_DL[g]);if(seen.count(key))continue;
    freudenthal f(factors[g].simpleroots,highestRoot(factors[g].algebra.substr(0,1),factors[g].rank),key.second);
    Character c=f.getWeightMap();int d=0;for(const auto& w:c){if(w.second<0)return 5;d+=w.second;}
    if(d!=std::abs(field.Dimensions[g].Dimension))return 6;
    seen[key]=records.size();records.push_back({g,key.second,d,c});
  }
  std::ofstream out(argv[2]);out<<std::setprecision(std::numeric_limits<double>::max_digits10);
  out<<"{\"characters\":[";
  for(size_t i=0;i<records.size();++i){if(i)out<<',';const auto& r=records[i];out<<"{\"factor\":"<<r.factor<<",\"highest\":";json(out,r.highest);out<<",\"dimension\":"<<r.dimension<<",\"weights\":";character(out,r.weights);out<<'}';}
  out<<"],\"pairs\":[";bool first=true;CGaugeInvariance gi;
  for(size_t i=0;i<records.size();++i)for(size_t j=i;j<records.size();++j){
    const auto& a=records[i];const auto& b=records[j];if(a.factor!=b.factor||a.dimension>32||b.dimension>32)continue;
    std::vector<std::vector<std::vector<int>>> tensor,symmetric,antisymmetric;
    const auto aw=expanded(a.weights),bw=expanded(b.weights);Character product;
    for(const auto& x:aw)for(const auto& y:bw){std::vector<int> sum(x.size());for(size_t k=0;k<sum.size();++k)sum[k]=x[k]+y[k];++product[sum];}
    const bool complete=gi.TensorProduct(aw,bw,factors[a.factor],tensor)&&count(tensor)==size_t(a.dimension)*size_t(b.dimension)&&joined(tensor)==product;
    if(!first)out<<',';first=false;out<<"{\"left\":"<<i<<",\"right\":"<<j<<",\"tensor_character\":";character(out,product);out<<",\"native_decomposition_complete\":"<<(complete?"true":"false");
    if(complete){out<<",\"tensor\":";decomposed(out,tensor);}
    if(i==j){
      Character sym,anti;
      for(size_t x=0;x<aw.size();++x)for(size_t y=x;y<aw.size();++y){std::vector<int> sum(aw[x].size());for(size_t k=0;k<sum.size();++k)sum[k]=aw[x][k]+aw[y][k];++sym[sum];if(x!=y)++anti[sum];}
      out<<",\"symmetric_character\":";character(out,sym);out<<",\"antisymmetric_character\":";character(out,anti);
      const bool self_complete=gi.TensorProductWithItself(aw,factors[a.factor],symmetric,antisymmetric)&&count(symmetric)==size_t(a.dimension)*(a.dimension+1)/2&&count(antisymmetric)==size_t(a.dimension)*(a.dimension-1)/2&&joined(symmetric)==sym&&joined(antisymmetric)==anti;
      out<<",\"native_self_decomposition_complete\":"<<(self_complete?"true":"false");
      if(self_complete){out<<",\"symmetric\":";decomposed(out,symmetric);out<<",\"antisymmetric\":";decomposed(out,antisymmetric);}
      if(a.dimension<=8){
        const auto w=expanded(a.weights);std::vector<std::vector<int>> space;
        for(size_t x=0;x<w.size();++x)for(size_t y=x;y<w.size();++y)for(size_t z=y;z<w.size();++z){std::vector<int> sum(w[x].size());for(size_t k=0;k<sum.size();++k)sum[k]=w[x][k]+w[y][k]+w[z][k];space.push_back(sum);}
        Character cube;for(const auto& weight:space)++cube[weight];
        out<<",\"symmetric_cube_character\":";character(out,cube);
      }
    }out<<'}';
  }
  out<<"]}\n";return out?0:12;
}
