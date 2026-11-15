use orbifolder_core::execution::{Executor, compute};
use orbifolder_data::{ModelData, canonical_result};
use serde_json::Value;
fn requests() -> Vec<orbifolder_core::execution::ModelRequest> {
    let data: Value =
        serde_json::from_str(include_str!("../../../tests/python/model_inputs.json")).unwrap();
    data.as_object()
        .unwrap()
        .values()
        .map(|v| {
            serde_json::from_value::<ModelData>(v["model"].clone())
                .unwrap()
                .request(None)
                .unwrap()
        })
        .collect()
}
#[test]
fn all_seven_models_errors_and_ids_are_identical_at_1_2_4_8_threads() {
    let mut input = requests();
    let mut bad = input[0].clone();
    bad.embedding.wilson_lines[0] =
        orbifolder_core::math_core::GaugeVector::new(std::array::from_fn(|i| {
            if i == 0 {
                orbifolder_core::math_core::Rational::new(1, 997).unwrap()
            } else {
                0.into()
            }
        }));
    input.push(bad);
    let mut capped = input[1].clone();
    capped.options.limits.search_nodes = 0;
    input.push(capped);
    let original = input.iter().map(compute).collect::<Vec<_>>();
    assert!(original[..7].iter().all(Result::is_ok));
    assert!(original[7..].iter().all(Result::is_err));
    let canonical = original.iter().map(canonical_result).collect::<Vec<_>>();
    for n in [1, 2, 4, 8] {
        let actual = Executor::parallel(n).unwrap().compute_many(&input);
        assert_eq!(
            canonical,
            actual.iter().map(canonical_result).collect::<Vec<_>>(),
            "pool size {n}"
        );
        for (a, e) in actual.iter().zip(&original) {
            if let (Err(a), Err(e)) = (a, e) {
                assert_eq!(a, e);
            }
        }
    }
    fn send_sync<T: Send + Sync>() {}
    send_sync::<orbifolder_core::execution::ModelResult>();
    send_sync::<Executor>();
    assert!(Executor::parallel(0).is_err());
}
#[test]
fn simultaneous_callers_reuse_one_pool_and_immutable_geometry() {
    let input = requests();
    let executor = Executor::parallel(2).unwrap();
    let baseline = executor
        .compute_many(&input)
        .iter()
        .map(canonical_result)
        .collect::<Vec<_>>();
    std::thread::scope(|scope| {
        let jobs = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    executor
                        .compute_many(&input)
                        .iter()
                        .map(canonical_result)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        for job in jobs {
            assert_eq!(job.join().unwrap(), baseline);
        }
    });
}
