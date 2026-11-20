//! A separate test process isolates the native build counter from other geometry tests.
#![cfg(not(feature = "extension-module"))]

use orbifolder_core::geometry::Geometry;
use pyo3::{ffi::c_str, prelude::*, types::PyDict, wrap_pymodule};

#[test]
fn prepared_batch_builds_geometry_exactly_once() {
    Python::initialize();
    Python::attach(|py| {
        let globals = PyDict::new(py);
        globals
            .set_item("orbifolder", wrap_pymodule!(_native::_native)(py))
            .unwrap();
        let before = Geometry::test_build_count();
        py.run(
            c_str!(
                r#"
geometry = orbifolder.Geometry.from_preset("Z3")
zero = [0] * 16
embedding = dict(lattice="E8xE8", wilson_lines=[zero] * 6)
shift = ["1/3", "1/3", "-2/3"] + [0] * 13
conjugate = ["-1/3", "-1/3", "2/3"] + [0] * 13
items = [
    geometry.build_model(label="model", shifts=[shift, zero, zero], **embedding),
    geometry.request(label="request", shifts=[conjugate, zero, zero], **embedding),
]
rows = orbifolder.Executor(threads=2).compute_summaries(items)
assert [row["status"] for row in rows] == ["ok", "ok"], rows
assert [row["summary"]["label"] for row in rows] == ["model", "request"]
"#
            ),
            Some(&globals),
            None,
        )
        .unwrap();
        assert_eq!(Geometry::test_build_count() - before, 1);
    });
}
