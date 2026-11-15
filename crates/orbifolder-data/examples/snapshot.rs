use orbifolder_core::execution::{Executor, compute};
use orbifolder_data::{ModelData, canonical_result};
use serde_json::Value;
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let data: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let requests = data
        .as_object()
        .unwrap()
        .values()
        .map(|e| {
            serde_json::from_value::<ModelData>(e["model"].clone())
                .unwrap()
                .request(None)
                .unwrap()
        })
        .collect::<Vec<_>>();
    if args.get(2).is_some_and(|x| x == "bench") {
        let repeats = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
        let mut rows = Vec::new();
        for threads in [0, 1, 2, 4, 8] {
            let executor = if threads == 0 {
                Executor::serial()
            } else {
                Executor::parallel(threads).unwrap()
            };
            let mut samples = Vec::new();
            for _ in 0..repeats {
                let start = std::time::Instant::now();
                let r = executor.compute_many(&requests);
                let time = start.elapsed().as_secs_f64();
                assert!(r.iter().all(Result::is_ok));
                std::hint::black_box(r);
                samples.push(time);
            }
            rows.push(serde_json::json!({"threads":threads,"seconds":samples}));
        }
        // A shared-geometry background batch, distinct from the seven-case timing.
        let backgrounds = (0..1000)
            .map(|i| requests[i % requests.len()].clone())
            .collect::<Vec<_>>();
        let executor = Executor::parallel(8).unwrap();
        let start = std::time::Instant::now();
        let mut fields = 0;
        for chunk in backgrounds.chunks(32) {
            let r = executor.compute_many(chunk);
            assert!(r.iter().all(Result::is_ok));
            fields += r
                .iter()
                .map(|r| r.as_ref().unwrap().spectrum().fields().len())
                .sum::<usize>();
            std::hint::black_box(r);
        }
        let seconds = start.elapsed().as_secs_f64();
        let reference = Executor::serial().compute_many(&requests);
        let export_start = std::time::Instant::now();
        let canonical_bytes = reference
            .iter()
            .map(|r| canonical_result(r).len())
            .sum::<usize>();
        let export_seconds = export_start.elapsed().as_secs_f64();
        let values = reference
            .iter()
            .flat_map(|r| r.as_ref().unwrap().spectrum().fields())
            .flat_map(|f| f.highest_weight().coordinates())
            .collect::<Vec<_>>();
        let big_values = values.iter().filter(|r| r.uses_bigint()).count();
        println!(
            "{}",
            serde_json::json!({"rust":rows,"background_batch":{"models":1000,"threads":8,"chunk_size":32,"massless_fields":fields,"seconds":seconds},"canonical_export":{"seconds":export_seconds,"bytes":canonical_bytes},"returned_highest_weight_values":{"total":values.len(),"bigint_backend":big_values}})
        );
    } else {
        let results = requests
            .iter()
            .map(|r| serde_json::from_str::<Value>(&canonical_result(&compute(r))).unwrap())
            .collect::<Vec<_>>();
        let keys = data.as_object().unwrap().keys();
        let output = keys
            .cloned()
            .zip(results)
            .collect::<serde_json::Map<_, _>>();
        println!("{}", Value::Object(output));
    }
}
