use std::ops::Range;

use lambdaworks_math::field::fields::{
    fft_friendly::stark_252_prime_field::Stark252PrimeField, u64_prime_field::FE17,
};
use lambdaworks_stark::air::example::fibonacci_rap::{fibonacci_rap_trace, FibonacciRAP};
use lambdaworks_stark::air::example::{
    dummy_air, fibonacci_2_columns, fibonacci_f17, quadratic_air, simple_fibonacci,
};
use lambdaworks_stark::cairo_run::run::{generate_prover_args, program_path};
use lambdaworks_stark::{
    air::context::{AirContext, ProofOptions},
    fri::FieldElement,
    prover::prove,
    verifier::verify,
};

pub type FE = FieldElement<Stark252PrimeField>;

#[test_log::test]
fn test_prove_fib() {
    let trace = simple_fibonacci::fibonacci_trace([FE::from(1), FE::from(1)], 8);
    let trace_length = trace.n_rows();

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 1,
            coset_offset: 3,
        },
        trace_length,
        trace_columns: 1,
        transition_degrees: vec![1],
        transition_exemptions: vec![2],
        transition_offsets: vec![0, 1, 2],
        num_transition_constraints: 1,
    };

    let fibonacci_air = simple_fibonacci::FibonacciAIR::from(context);

    let result = prove(&trace, &fibonacci_air, &mut ()).unwrap();
    assert!(verify(&result, &fibonacci_air, &()));
}

#[test_log::test]
fn test_prove_fib17() {
    let trace = simple_fibonacci::fibonacci_trace([FE17::from(1), FE17::from(1)], 4);
    let trace_length = trace.n_rows();

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 1,
            coset_offset: 3,
        },
        trace_length,
        trace_columns: 1,
        transition_degrees: vec![1],
        transition_exemptions: vec![2],
        transition_offsets: vec![0, 1, 2],
        num_transition_constraints: 1,
    };

    let fibonacci_air = fibonacci_f17::Fibonacci17AIR::from(context);

    let result = prove(&trace, &fibonacci_air, &mut ()).unwrap();
    assert!(verify(&result, &fibonacci_air, &()));
}

#[test_log::test]
fn test_prove_fib_2_cols() {
    let trace = fibonacci_2_columns::fibonacci_trace_2_columns([FE::from(1), FE::from(1)], 16);
    let trace_length = trace.n_rows();

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 7,
            coset_offset: 3,
        },
        trace_length,
        transition_degrees: vec![1, 1],
        transition_exemptions: vec![1, 1],
        transition_offsets: vec![0, 1],
        num_transition_constraints: 2,
        trace_columns: 2,
    };

    let fibonacci_air = fibonacci_2_columns::Fibonacci2ColsAIR::from(context);

    let result = prove(&trace, &fibonacci_air, &mut ()).unwrap();
    assert!(verify(&result, &fibonacci_air, &()));
}

#[test_log::test]
fn test_prove_quadratic() {
    let trace = quadratic_air::quadratic_trace(FE::from(3), 4);
    let trace_length = trace.n_rows();

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 1,
            coset_offset: 3,
        },
        trace_length,
        trace_columns: 1,
        transition_degrees: vec![2],
        transition_exemptions: vec![1],
        transition_offsets: vec![0, 1],
        num_transition_constraints: 1,
    };

    let quadratic_air = quadratic_air::QuadraticAIR::from(context);

    let result = prove(&trace, &quadratic_air, &mut ()).unwrap();
    assert!(verify(&result, &quadratic_air, &()));
}

#[ignore = "metal"]
/// Loads the program in path, runs it with the Cairo VM, and amkes a proof of it
fn test_prove_cairo_program(file_path: &str, rc_builtin_range: Option<Range<u64>>) {
    let (main_trace, cairo_air, mut pub_inputs) = generate_prover_args(file_path, rc_builtin_range);
    let result = prove(&main_trace, &cairo_air, &mut pub_inputs).unwrap();

    assert!(verify(&result, &cairo_air, &pub_inputs));
}

#[test_log::test]
fn test_prove_cairo_simple_program() {
    test_prove_cairo_program(&program_path("simple_program.json"), None);
}

#[test_log::test]
fn test_prove_cairo_fibonacci_5() {
    test_prove_cairo_program(&program_path("fibonacci_5.json"), None);
}

#[test_log::test]
fn test_prove_cairo_rc_program() {
    test_prove_cairo_program(&program_path("rc_program.json"), Some(25..26));
}

#[test_log::test]
fn test_prove_cairo_lt_comparison() {
    test_prove_cairo_program(&program_path("lt_comparison.json"), Some(131..132));
}

// #[test_log::test]
// fn test_prove_cairo_compare_lesser_array() {
//     test_prove_cairo_program(&program_path("compare_lesser_array.json"), Some(2400..2410));
// }

#[test_log::test]
fn test_prove_rap_fib() {
    let trace_length = 16;
    let trace = fibonacci_rap_trace([FE::from(1), FE::from(1)], trace_length);
    let trace_cols = trace.cols();
    let power_of_two_len = trace_cols[0].len();
    let exemptions = 3 + power_of_two_len - trace_length - 1;

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 1,
            coset_offset: 3,
        },
        trace_columns: 3,
        trace_length: trace_cols[0].len(),
        transition_degrees: vec![1, 2],
        transition_offsets: vec![0, 1, 2],
        transition_exemptions: vec![exemptions, 1],
        num_transition_constraints: 2,
    };

    let fibonacci_rap = FibonacciRAP::new(context);

    let result = prove(&trace, &fibonacci_rap, &mut ()).unwrap();
    assert!(verify(&result, &fibonacci_rap, &()));
}

#[test_log::test]
fn test_prove_dummy() {
    let trace_length = 16;
    let trace = dummy_air::dummy_trace(trace_length);

    let context = AirContext {
        options: ProofOptions {
            blowup_factor: 2,
            fri_number_of_queries: 1,
            coset_offset: 3,
        },
        trace_length,
        trace_columns: 2,
        transition_degrees: vec![2, 1],
        transition_exemptions: vec![0, 2],
        transition_offsets: vec![0, 1, 2],
        num_transition_constraints: 2,
    };

    let dummy_air = dummy_air::DummyAIR::from(context);

    let result = prove(&trace, &dummy_air, &mut ()).unwrap();
    assert!(verify(&result, &dummy_air, &()));
}

#[test_log::test]
fn test_verifier_rejects_proof_of_a_slightly_different_program() {
    let (main_trace, cairo_air, mut public_input) =
        generate_prover_args(&program_path("simple_program.json"), None);
    let result = prove(&main_trace, &cairo_air, &mut public_input).unwrap();

    // We modify the original program and verify using this new "corrupted" version
    let mut corrupted_program = public_input.program.clone();
    corrupted_program[1] = FieldElement::from(5);
    corrupted_program[3] = FieldElement::from(5);

    // Here we use the corrupted version of the program in the public inputs
    public_input.program = corrupted_program;
    assert!(!verify(&result, &cairo_air, &public_input));
}

#[test_log::test]
fn test_verifier_rejects_proof_with_different_range_bounds() {
    let (main_trace, cairo_air, mut public_input) =
        generate_prover_args(&program_path("simple_program.json"), None);
    let result = prove(&main_trace, &cairo_air, &mut public_input).unwrap();

    public_input.range_check_min = Some(public_input.range_check_min.unwrap() + 1);
    assert!(!verify(&result, &cairo_air, &public_input));

    public_input.range_check_min = Some(public_input.range_check_min.unwrap() - 1);
    public_input.range_check_max = Some(public_input.range_check_max.unwrap() - 1);
    assert!(!verify(&result, &cairo_air, &public_input));
}
