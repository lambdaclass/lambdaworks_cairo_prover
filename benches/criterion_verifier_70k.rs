use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode,
};
use functions::{execution::run_verifier_bench_with_security_level, path::cairo0_proof_path};

use lambdaworks_stark::starks::proof::options::SecurityLevel;

pub mod functions;

fn verifier_benches(c: &mut Criterion) {
    #[cfg(feature = "parallel")]
    {
        let num_threads: usize = std::env::var("NUM_THREADS")
            .unwrap_or("8".to_string())
            .parse()
            .unwrap();
        println!("Running benchmarks using {} threads", num_threads);
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .unwrap();
    };

    let mut group = c.benchmark_group("VERIFIER");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);
    run_verifier_bench(
        &mut group,
        "fibonacci/70000",
        &cairo0_proof_path("fibonacci_70000_sec.proof"),
    );
}

fn run_verifier_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
) {
    run_verifier_bench_with_security_level(
        group,
        benchname,
        program_path,
        SecurityLevel::Conjecturable128Bits,
    );
}

criterion_group!(benches, verifier_benches);
criterion_main!(benches);
