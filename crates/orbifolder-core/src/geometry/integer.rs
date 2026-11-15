//! Integer image/kernel certificates U*A*V=D, with unimodular U and V.
use super::ModelError;
use crate::math_core::{Matrix, Rational};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, Zero};

#[derive(Clone, Debug)]
pub(super) struct Smith {
    pub diagonal: Vec<BigInt>,
    pub left: Matrix,
    pub right: Matrix,
    pub rank: usize,
    pub rows: usize,
    pub cols: usize,
}

fn row_add(a: &mut [Vec<BigInt>], target: usize, source: usize, q: &BigInt) {
    for j in 0..a[target].len() {
        let x = &a[source][j] * q;
        a[target][j] += &x;
    }
}
fn col_add(a: &mut [Vec<BigInt>], target: usize, source: usize, q: &BigInt) {
    for row in a {
        let x = &row[source] * q;
        row[target] += &x;
    }
}
fn swap_cols(a: &mut [Vec<BigInt>], i: usize, j: usize) {
    for row in a {
        row.swap(i, j);
    }
}

impl Smith {
    pub fn new(matrix: &Matrix) -> Result<Self, ModelError> {
        let rows = matrix.rows();
        let cols = matrix.cols();
        let mut a = (0..rows)
            .map(|i| {
                (0..cols)
                    .map(|j| matrix.get(i, j).unwrap().to_integer())
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let identity = |n: usize| {
            (0..n)
                .map(|i| (0..n).map(|j| BigInt::from(i == j)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        let mut u = identity(rows);
        let mut v = identity(cols);
        let mut rank = 0;
        for k in 0..rows.min(cols) {
            let pivot = (k..rows)
                .flat_map(|i| (k..cols).map(move |j| (i, j)))
                .filter(|&(i, j)| !a[i][j].is_zero())
                .min_by_key(|&(i, j)| a[i][j].abs());
            let Some((i, j)) = pivot else { break };
            a.swap(k, i);
            u.swap(k, i);
            swap_cols(&mut a, k, j);
            swap_cols(&mut v, k, j);
            loop {
                let mut changed = false;
                for i in k + 1..rows {
                    if a[i][k].is_zero() {
                        continue;
                    }
                    let q = -(&a[i][k] / &a[k][k]);
                    row_add(&mut a, i, k, &q);
                    row_add(&mut u, i, k, &q);
                    if !a[i][k].is_zero() {
                        a.swap(i, k);
                        u.swap(i, k);
                    }
                    changed = true;
                    break;
                }
                if changed {
                    continue;
                }
                for j in k + 1..cols {
                    if a[k][j].is_zero() {
                        continue;
                    }
                    let q = -(&a[k][j] / &a[k][k]);
                    col_add(&mut a, j, k, &q);
                    col_add(&mut v, j, k, &q);
                    if !a[k][j].is_zero() {
                        swap_cols(&mut a, j, k);
                        swap_cols(&mut v, j, k);
                    }
                    changed = true;
                    break;
                }
                if changed {
                    continue;
                }
                let bad = (k + 1..rows)
                    .find(|&i| (k + 1..cols).any(|j| !a[i][j].mod_floor(&a[k][k]).is_zero()));
                if let Some(i) = bad {
                    row_add(&mut a, k, i, &BigInt::from(1));
                    row_add(&mut u, k, i, &BigInt::from(1));
                    continue;
                }
                break;
            }
            if a[k][k].is_negative() {
                for x in &mut a[k] {
                    *x = -x.clone();
                }
                for x in &mut u[k] {
                    *x = -x.clone();
                }
            }
            rank += 1;
        }
        let convert = |m: Vec<Vec<BigInt>>, r, c| {
            Matrix::new(r, c, m.into_iter().flatten().map(Rational::from).collect())
        };
        Ok(Self {
            diagonal: (0..rows.min(cols)).map(|i| a[i][i].clone()).collect(),
            left: convert(u, rows, rows)?,
            right: convert(v, cols, cols)?,
            rank,
            rows,
            cols,
        })
    }
    pub fn solve(&self, rhs: &[Rational]) -> Result<Option<Vec<Rational>>, ModelError> {
        if rhs.len() != self.rows {
            return Err(ModelError::InvalidElement);
        }
        let y = self.left.apply(rhs)?;
        let mut z = vec![Rational::ZERO; self.cols];
        for (i, x) in y.iter().enumerate() {
            if i >= self.rank {
                if !x.is_zero() {
                    return Ok(None);
                }
            } else {
                let q = x.checked_div(&Rational::from(self.diagonal[i].clone()))?;
                if !q.is_integer() {
                    return Ok(None);
                }
                z[i] = q;
            }
        }
        Ok(Some(self.right.apply(&z)?))
    }
    pub fn kernel(&self) -> Vec<Vec<Rational>> {
        (self.rank..self.cols)
            .map(|j| {
                (0..self.cols)
                    .map(|i| self.right.get(i, j).unwrap().clone())
                    .collect()
            })
            .collect()
    }
    pub fn torsion_key(&self, vector: &[Rational]) -> Result<Option<Vec<BigInt>>, ModelError> {
        let y = self.left.apply(vector)?;
        let mut key = Vec::new();
        for (i, x) in y.iter().enumerate() {
            if !x.is_integer() {
                return Ok(None);
            }
            key.push(if i < self.rank {
                x.to_integer()?.mod_floor(&self.diagonal[i])
            } else {
                x.to_integer()?
            });
        }
        Ok(Some(key))
    }
    pub fn quotient_order(&self, vector: &[Rational]) -> Result<Option<BigInt>, ModelError> {
        let y = self.left.apply(vector)?;
        if y.iter().skip(self.rank).any(|v| !v.is_zero()) {
            return Ok(None);
        }
        let mut order = BigInt::from(1);
        for (i, x) in y.iter().take(self.rank).enumerate() {
            let x = x.to_integer()?;
            let d = &self.diagonal[i];
            order = order.lcm(&(d / d.gcd(&x)));
        }
        Ok(Some(order))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn smith_certificates(values in prop::collection::vec(-20i64..20,12)) {
            let a=Matrix::new(3,4,values.into_iter().map(Rational::from).collect()).unwrap();
            let s=Smith::new(&a).unwrap();
            let diagonal=s.left.multiply(&a).unwrap().multiply(&s.right).unwrap();
            for (i, value) in s.diagonal.iter().enumerate() {
                for j in 0..4 {
                    prop_assert_eq!(diagonal.get(i,j).unwrap(), &if i==j {Rational::from(value.clone())} else {Rational::ZERO});
                }
            }
            prop_assert_eq!(s.left.determinant().unwrap().abs(),Rational::ONE);
            prop_assert_eq!(s.right.determinant().unwrap().abs(),Rational::ONE);
            for v in s.kernel(){prop_assert!(a.apply(&v).unwrap().iter().all(Rational::is_zero));}
            for i in 1..s.rank {prop_assert!(s.diagonal[i].mod_floor(&s.diagonal[i-1]).is_zero());}
        }
    }
    #[test]
    fn primitive_kernel_and_integer_solvability() {
        let a = Matrix::new(1, 2, vec![2.into(), 4.into()]).unwrap();
        let s = Smith::new(&a).unwrap();
        assert!(s.solve(&[1.into()]).unwrap().is_none());
        assert!(s.solve(&[2.into()]).unwrap().is_some());
        assert_eq!(s.kernel().len(), 1);
        assert_eq!(a.apply(&s.kernel()[0]).unwrap(), vec![Rational::ZERO]);
        assert_eq!(s.quotient_order(&[1.into()]).unwrap(), Some(2.into()));
        assert_eq!(s.torsion_key(&[3.into()]).unwrap(), Some(vec![1.into()]));
    }
}
