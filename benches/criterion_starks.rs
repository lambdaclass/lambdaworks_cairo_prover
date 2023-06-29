use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use lambdaworks_stark::{
    cairo::runner::run::{generate_prover_args, CairoVersion},
    starks::{prover::prove, verifier::verify},
};
use std::time::Duration;

pub mod functions;

fn cairo_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("CAIRO");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    run_cairo_bench(
        &mut group,
        "fibonacci/10",
        &cairo0_program_path("fibonacci_10.json"),
    );
    run_cairo_bench(
        &mut group,
        "fibonacci/100",
        &cairo0_program_path("fibonacci_100.json"),
    );
}

fn cairo0_program_path(program_name: &str) -> String {
    const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");
    const PROGRAM_BASE_REL_PATH: &str = "/cairo_programs/cairo0/";
    let program_base_path = CARGO_DIR.to_string() + PROGRAM_BASE_REL_PATH;
    program_base_path + program_name
}

fn run_cairo_bench(group: &mut BenchmarkGroup<'_, WallTime>, benchname: &str, program_path: &str) {
    let program_content = std::fs::read(program_path).unwrap();
    let (main_trace, cairo_air, mut pub_inputs) =
        generate_prover_args(&program_content, &CairoVersion::V0, &None, 1).unwrap();

    group.bench_function(benchname, |bench| {
        bench.iter(|| black_box(prove(&main_trace, &cairo_air, &mut pub_inputs).unwrap()));
    });
}

fn verifier_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("VERIFIER");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    run_verifier_bench(
        &mut group,
        "fibonacci/10",
        &cairo0_program_path("fibonacci_10.json"),
    );
    run_verifier_bench(
        &mut group,
        "fibonacci/100",
        &cairo0_program_path("fibonacci_100.json"),
    );
}

fn run_verifier_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
) {
    let program_content = std::fs::read(program_path).unwrap();
    let (main_trace, cairo_air, mut pub_inputs) =
        generate_prover_args(&program_content, &CairoVersion::V0, &None, 1).unwrap();
    let proof = prove(&main_trace, &cairo_air, &mut pub_inputs).unwrap();

    group.bench_function(benchname, |bench| {
        bench.iter(|| black_box(verify(&proof, &cairo_air, &pub_inputs)));
    });
}

criterion_group!(benches, cairo_benches, verifier_benches);
criterion_main!(benches);
