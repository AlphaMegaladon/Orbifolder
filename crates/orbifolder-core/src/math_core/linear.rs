use super::{MathError, Rational};

/// Fixed-dimensional immutable coordinates. No implicit modulo reduction.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Vector<const N: usize>([Rational; N]);
pub type GaugeVector = Vector<16>;
pub type TorusVector = Vector<6>;
pub type RightWeight = Vector<4>;

impl<const N: usize> Vector<N> {
    pub fn new(coordinates: [Rational; N]) -> Self {
        Self(coordinates)
    }
    pub fn zero() -> Self {
        Self(std::array::from_fn(|_| Rational::ZERO))
    }
    pub fn from_integers(coordinates: [i128; N]) -> Self {
        Self(coordinates.map(Rational::from))
    }
    pub fn from_slice(coordinates: &[Rational]) -> Result<Self, MathError> {
        Ok(Self(
            coordinates
                .to_vec()
                .try_into()
                .map_err(|_| MathError::DimensionMismatch)?,
        ))
    }
    pub fn coordinates(&self) -> &[Rational; N] {
        &self.0
    }
    pub fn dot(&self, other: &Self) -> Rational {
        self.0
            .iter()
            .zip(&other.0)
            .fold(Rational::ZERO, |sum, (a, b)| sum + a * b)
    }
    pub fn norm_squared(&self) -> Rational {
        self.dot(self)
    }
    pub fn add(&self, other: &Self) -> Self {
        Self(std::array::from_fn(|i| &self.0[i] + &other.0[i]))
    }
    pub fn sub(&self, other: &Self) -> Self {
        Self(std::array::from_fn(|i| &self.0[i] - &other.0[i]))
    }
    pub fn scale(&self, scalar: &Rational) -> Self {
        Self(std::array::from_fn(|i| &self.0[i] * scalar))
    }
}

pub fn weyl_reflect<const N: usize>(
    weight: &Vector<N>,
    root: &Vector<N>,
) -> Result<Vector<N>, MathError> {
    let norm = root.norm_squared();
    if norm.is_zero() {
        return Err(MathError::InvalidRootSystem);
    }
    let coefficient = (Rational::from(2) * weight.dot(root)).checked_div(&norm)?;
    Ok(weight.sub(&root.scale(&coefficient)))
}

/// A checked rectangular matrix. Shape is retained even for zero rows/columns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    entries: Vec<Rational>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearSolution {
    particular: Vec<Rational>,
    /// Directions of the homogeneous solution, one vector per free variable.
    nullspace: Vec<Vec<Rational>>,
}

impl LinearSolution {
    pub fn particular(&self) -> &[Rational] {
        &self.particular
    }
    pub fn nullspace(&self) -> &[Vec<Rational>] {
        &self.nullspace
    }
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, entries: Vec<Rational>) -> Result<Self, MathError> {
        let length = rows.checked_mul(cols).ok_or(MathError::CapacityOverflow)?;
        if length != entries.len() {
            return Err(MathError::DimensionMismatch);
        }
        Ok(Self {
            rows,
            cols,
            entries,
        })
    }
    pub fn from_rows(rows: Vec<Vec<Rational>>) -> Result<Self, MathError> {
        let cols = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|r| r.len() != cols) {
            return Err(MathError::DimensionMismatch);
        }
        Self::new(rows.len(), cols, rows.into_iter().flatten().collect())
    }
    pub fn zeros(rows: usize, cols: usize) -> Result<Self, MathError> {
        let length = rows.checked_mul(cols).ok_or(MathError::CapacityOverflow)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(length)
            .map_err(|_| MathError::CapacityOverflow)?;
        entries.resize(length, Rational::ZERO);
        Self::new(rows, cols, entries)
    }
    pub fn identity(n: usize) -> Result<Self, MathError> {
        let mut result = Self::zeros(n, n)?;
        for i in 0..n {
            result.set(i, i, Rational::ONE);
        }
        Ok(result)
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cols(&self) -> usize {
        self.cols
    }
    pub fn entries(&self) -> &[Rational] {
        &self.entries
    }
    pub fn get(&self, row: usize, col: usize) -> Option<&Rational> {
        if row >= self.rows || col >= self.cols {
            None
        } else {
            Some(self.at(row, col))
        }
    }
    fn at(&self, row: usize, col: usize) -> &Rational {
        &self.entries[row * self.cols + col]
    }
    fn set(&mut self, row: usize, col: usize, value: Rational) {
        self.entries[row * self.cols + col] = value;
    }
    fn swap_rows(&mut self, a: usize, b: usize) {
        for col in 0..self.cols {
            self.entries.swap(a * self.cols + col, b * self.cols + col);
        }
    }
    pub fn transpose(&self) -> Self {
        let mut entries = Vec::with_capacity(self.entries.len());
        for j in 0..self.cols {
            for i in 0..self.rows {
                entries.push(self.at(i, j).clone());
            }
        }
        Self {
            rows: self.cols,
            cols: self.rows,
            entries,
        }
    }
    pub fn multiply(&self, other: &Self) -> Result<Self, MathError> {
        if self.cols != other.rows {
            return Err(MathError::DimensionMismatch);
        }
        let mut result = Self::zeros(self.rows, other.cols)?;
        for i in 0..self.rows {
            for k in 0..self.cols {
                if self.at(i, k).is_zero() {
                    continue;
                }
                for j in 0..other.cols {
                    result.set(i, j, result.at(i, j) + self.at(i, k) * other.at(k, j));
                }
            }
        }
        Ok(result)
    }
    pub fn apply(&self, vector: &[Rational]) -> Result<Vec<Rational>, MathError> {
        if vector.len() != self.cols {
            return Err(MathError::DimensionMismatch);
        }
        Ok((0..self.rows)
            .map(|i| {
                vector
                    .iter()
                    .enumerate()
                    .fold(Rational::ZERO, |sum, (j, x)| sum + self.at(i, j) * x)
            })
            .collect())
    }
    pub fn apply_vector<const N: usize>(&self, vector: &Vector<N>) -> Result<Vector<N>, MathError> {
        Vector::from_slice(&self.apply(vector.coordinates())?)
    }
    pub fn bilinear(&self, left: &[Rational], right: &[Rational]) -> Result<Rational, MathError> {
        if left.len() != self.rows {
            return Err(MathError::DimensionMismatch);
        }
        Ok(left
            .iter()
            .zip(self.apply(right)?)
            .fold(Rational::ZERO, |sum, (x, y)| sum + x * y))
    }
    pub fn rref(&self) -> Result<(Self, Vec<usize>), MathError> {
        let mut result = self.clone();
        let mut pivots = Vec::new();
        for col in 0..self.cols {
            let row = pivots.len();
            if row == self.rows {
                break;
            }
            let Some(pivot) = (row..self.rows).find(|&r| !result.at(r, col).is_zero()) else {
                continue;
            };
            result.swap_rows(row, pivot);
            let divisor = result.at(row, col).clone();
            for j in 0..self.cols {
                result.set(row, j, result.at(row, j).checked_div(&divisor)?);
            }
            for i in 0..self.rows {
                if i == row {
                    continue;
                }
                let factor = result.at(i, col).clone();
                if factor.is_zero() {
                    continue;
                }
                for j in 0..self.cols {
                    result.set(i, j, result.at(i, j) - &factor * result.at(row, j));
                }
            }
            pivots.push(col);
        }
        Ok((result, pivots))
    }
    pub fn rank(&self) -> Result<usize, MathError> {
        Ok(self.rref()?.1.len())
    }
    pub fn nullspace(&self) -> Result<Vec<Vec<Rational>>, MathError> {
        let (rref, pivots) = self.rref()?;
        Ok((0..self.cols)
            .filter(|i| !pivots.contains(i))
            .map(|free| {
                let mut vector = vec![Rational::ZERO; self.cols];
                vector[free] = Rational::ONE;
                for (row, &pivot) in pivots.iter().enumerate() {
                    vector[pivot] = -rref.at(row, free);
                }
                vector
            })
            .collect())
    }
    pub fn solve(&self, rhs: &[Rational]) -> Result<LinearSolution, MathError> {
        if rhs.len() != self.rows {
            return Err(MathError::DimensionMismatch);
        }
        let columns = self
            .cols
            .checked_add(1)
            .ok_or(MathError::CapacityOverflow)?;
        let mut augmented = Self::zeros(self.rows, columns)?;
        for (i, b) in rhs.iter().enumerate() {
            for j in 0..self.cols {
                augmented.set(i, j, self.at(i, j).clone());
            }
            augmented.set(i, self.cols, b.clone());
        }
        let (reduced, pivots) = augmented.rref()?;
        if pivots.contains(&self.cols) {
            return Err(MathError::InconsistentSystem);
        }
        let mut particular = vec![Rational::ZERO; self.cols];
        for (row, col) in pivots.into_iter().enumerate() {
            particular[col] = reduced.at(row, self.cols).clone();
        }
        Ok(LinearSolution {
            particular,
            nullspace: self.nullspace()?,
        })
    }
    pub fn determinant(&self) -> Result<Rational, MathError> {
        if self.rows != self.cols {
            return Err(MathError::DimensionMismatch);
        }
        let mut matrix = self.clone();
        let mut determinant = Rational::ONE;
        for k in 0..self.rows {
            let Some(pivot) = (k..self.rows).find(|&i| !matrix.at(i, k).is_zero()) else {
                return Ok(Rational::ZERO);
            };
            if pivot != k {
                matrix.swap_rows(pivot, k);
                determinant = -determinant;
            }
            let divisor = matrix.at(k, k).clone();
            determinant = determinant * &divisor;
            for i in k + 1..self.rows {
                let factor = matrix.at(i, k).checked_div(&divisor)?;
                for j in k + 1..self.cols {
                    matrix.set(i, j, matrix.at(i, j) - &factor * matrix.at(k, j));
                }
            }
        }
        Ok(determinant)
    }
    pub fn inverse(&self) -> Result<Self, MathError> {
        if self.rows != self.cols {
            return Err(MathError::DimensionMismatch);
        }
        let width = self
            .cols
            .checked_mul(2)
            .ok_or(MathError::CapacityOverflow)?;
        let mut augmented = Self::zeros(self.rows, width)?;
        for i in 0..self.rows {
            for j in 0..self.cols {
                augmented.set(i, j, self.at(i, j).clone());
            }
            augmented.set(i, self.cols + i, Rational::ONE);
        }
        let (reduced, pivots) = augmented.rref()?;
        if pivots.len() < self.rows || pivots.iter().take(self.rows).copied().ne(0..self.rows) {
            return Err(MathError::SingularMatrix);
        }
        let mut result = Self::zeros(self.rows, self.cols)?;
        for j in 0..self.cols {
            for i in 0..self.rows {
                result.set(i, j, reduced.at(i, self.cols + j).clone());
            }
        }
        Ok(result)
    }
    /// Exact LDL elimination, with no numerical square roots.
    pub fn is_positive_definite(&self) -> bool {
        if self.rows != self.cols || self != &self.transpose() {
            return false;
        }
        let mut reduced = self.clone();
        for k in 0..self.rows {
            let pivot = reduced.at(k, k).clone();
            if pivot <= Rational::ZERO {
                return false;
            }
            for i in k + 1..self.rows {
                for j in k + 1..self.cols {
                    let correction = (reduced.at(i, k) * reduced.at(k, j))
                        .checked_div(&pivot)
                        .expect("positive pivot");
                    reduced.set(i, j, reduced.at(i, j) - correction);
                }
            }
        }
        true
    }
    pub fn is_isometry(&self, action: &Self) -> Result<bool, MathError> {
        if self.rows != self.cols || action.rows != self.rows || action.cols != self.cols {
            return Err(MathError::DimensionMismatch);
        }
        Ok(action.transpose().multiply(self)?.multiply(action)? == *self)
    }
}
