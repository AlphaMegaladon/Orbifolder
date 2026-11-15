# Scientific provenance and third-party notices

Copyright (C) 2026 Martin Wimmer for the Rust/Python implementation, adapters,
tests and documentation created for this project. Distributed under
**GPL-3.0-or-later**; see [LICENSE](./LICENSE). You may redistribute and/or modify
this project under GNU GPL version 3 or, at your option, any later version.
It is supplied without warranty, including merchantability or fitness for a
particular purpose. Upstream authors retain copyright in their respective work.

## Original SUSY orbifolder

H. P. Nilles, S. Ramos-Sánchez, P. K. S. Vaudrevange, A. Wingerter,
*The Orbifolder: A Tool to study the Low Energy Effective Theory of Heterotic
Orbifolds*, Computer Physics Communications **183** (2012), 1363–1380.
[arXiv:1110.5229](https://arxiv.org/abs/1110.5229),
[DOI:10.1016/j.cpc.2012.01.026](https://doi.org/10.1016/j.cpc.2012.01.026).

The reference implementation is C++ orbifolder **v1.2.1**, distributed via
[HepForge](https://orbifolder.hepforge.org/). Its archive includes the GNU GPL v3
license. Its mathematical algorithms, geometry/model data and native outputs
provided the foundation and independent oracle for this migration.

## non-SUSY orbifolder

E. Escalante-Notario, R. Perez-Martinez, S. Ramos-Sánchez,
P. K. S. Vaudrevange, *The Non-SUSY Orbifolder: a tool to build promising non-supersymmetric string models*,
[arXiv:2504.20137](https://arxiv.org/abs/2504.20137).
Source: [StringsIFUNAM/nonSUSYorbifolder](https://github.com/StringsIFUNAM/nonSUSYorbifolder).
The source archive includes the GNU GPL v3 license.

The native non-SUSY implementation extends the original orbifolder and supplies
the reference non-supersymmetric embeddings, projections and tachyon searches.
The initially supplied identifier `2505.00083` refers to an unrelated astronomy
paper; the identifier above is verified against the upstream README and arXiv.

## What is distributed here

The public repository contains the new Rust/Python implementation, our C++
**probe adapters** (which require separately obtained upstream headers/libraries),
structured geometry/model inputs, and captured numerical oracle results.
The upstream C++ source archives, binaries and runtime tables are **not included**
in the repository, wheel or source distribution. Locally they may reside in the
ignored legacy directory; see [oracle reproduction](./tests/ground_truth/README.md).

Input geometries, example embeddings and numerical fixtures are attributed to
the corresponding upstream projects; they are not claimed as original scientific
discoveries by this project. Fixture provenance includes source/input hashes.
Exact Gram matrices interpret the archived rounded torus bases in Q(sqrt(3));
that normalization and the independently corrected algorithms are documented in
the [verification reports](./docs/03_verification_and_migration_plan.md) and audits.
Removing upstream source files from Git does not remove these obligations or
the scientific lineage of this implementation. Please cite both predecessor
papers as well as this package when using it in research.

Rust dependencies retain their own licenses (including PyO3, Rayon and the
num/serde ecosystem). Resolved versions are recorded in [Cargo.lock](./Cargo.lock).

## Documented independent corrections

The Rust implementation replaces tolerance-based exact decisions, uninitialized
tachyon labels, incomplete Wilson congruences, fixed tachyon search windows,
premature/verbose-dependent anomaly success, incomplete tensor decomposition,
and unsymmetrized repeated-field candidate counts. See the linked audits in
[README](./README.md); the original seven C++ fixtures remain unchanged.
