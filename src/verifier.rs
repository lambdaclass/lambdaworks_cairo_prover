#[cfg(feature = "instruments")]
use std::time::Instant;

use super::{
    air::constraints::evaluator::ConstraintEvaluator, fri::fri_decommit::FriDecommitment,
    sample_z_ood,
};
use crate::{
    air::traits::AIR,
    batch_sample_challenges,
    fri::{Commitment, FriMerkleBackend},
    proof::StarkProof,
    transcript_to_field, transcript_to_usize, BatchStarkProverBackend, Domain,
};
#[cfg(not(feature = "test_fiat_shamir"))]
use lambdaworks_crypto::fiat_shamir::default_transcript::DefaultTranscript;
use lambdaworks_crypto::fiat_shamir::transcript::Transcript;
use log::error;

#[cfg(feature = "test_fiat_shamir")]
use lambdaworks_crypto::fiat_shamir::test_transcript::TestTranscript;

use lambdaworks_math::{
    field::{
        element::FieldElement,
        traits::{IsFFTField, IsField},
    },
    polynomial::Polynomial,
    traits::ByteConversion,
};

#[cfg(feature = "test_fiat_shamir")]
fn step_1_transcript_initialization() -> TestTranscript {
    TestTranscript::new()
}

#[cfg(not(feature = "test_fiat_shamir"))]
fn step_1_transcript_initialization() -> DefaultTranscript {
    // TODO: add strong fiat shamir
    DefaultTranscript::new()
}

struct Challenges<F: IsFFTField, A: AIR<Field = F>> {
    z: FieldElement<F>,
    boundary_coeffs: Vec<(FieldElement<F>, FieldElement<F>)>,
    transition_coeffs: Vec<(FieldElement<F>, FieldElement<F>)>,
    trace_term_coeffs: Vec<Vec<FieldElement<F>>>,
    gamma_even: FieldElement<F>,
    gamma_odd: FieldElement<F>,
    zetas: Vec<FieldElement<F>>,
    iotas: Vec<usize>,
    rap_challenges: A::RAPChallenges,
}

fn step_1_replay_rounds_and_recover_challenges<F, A, T>(
    air: &A,
    proof: &StarkProof<F>,
    domain: &Domain<F>,
    transcript: &mut T,
) -> Challenges<F, A>
where
    F: IsFFTField,
    FieldElement<F>: ByteConversion,
    A: AIR<Field = F>,
    T: Transcript,
{
    // ===================================
    // ==========|   Round 1   |==========
    // ===================================

    // <<<< Receive commitments:[tⱼ]
    let total_columns = air.context().trace_columns;

    transcript.append(&proof.lde_trace_merkle_roots[0]);

    let rap_challenges = air.build_rap_challenges(transcript);

    if let Some(root) = proof.lde_trace_merkle_roots.get(1) {
        transcript.append(root);
    }

    // ===================================
    // ==========|   Round 2   |==========
    // ===================================

    // These are the challenges alpha^B_j and beta^B_j
    // >>>> Send challenges: 𝛼_j^B
    let boundary_coeffs_alphas = batch_sample_challenges(total_columns, transcript);
    // >>>> Send  challenges: 𝛽_j^B
    let boundary_coeffs_betas = batch_sample_challenges(total_columns, transcript);
    // >>>> Send challenges: 𝛼_j^T
    let transition_coeffs_alphas =
        batch_sample_challenges(air.context().num_transition_constraints, transcript);
    // >>>> Send challenges: 𝛽_j^T
    let transition_coeffs_betas =
        batch_sample_challenges(air.context().num_transition_constraints, transcript);
    let boundary_coeffs: Vec<_> = boundary_coeffs_alphas
        .into_iter()
        .zip(boundary_coeffs_betas)
        .collect();

    let transition_coeffs: Vec<_> = transition_coeffs_alphas
        .into_iter()
        .zip(transition_coeffs_betas)
        .collect();

    // <<<< Receive commitments: [H₁], [H₂]
    transcript.append(&proof.composition_poly_root);

    // ===================================
    // ==========|   Round 3   |==========
    // ===================================

    // >>>> Send challenge: z
    let z = sample_z_ood(
        &domain.lde_roots_of_unity_coset,
        &domain.trace_roots_of_unity,
        transcript,
    );

    // <<<< Receive value: H₁(z²)
    transcript.append(&proof.composition_poly_even_ood_evaluation.to_bytes_be());
    // <<<< Receive value: H₂(z²)
    transcript.append(&proof.composition_poly_odd_ood_evaluation.to_bytes_be());
    // <<<< Receive values: tⱼ(zgᵏ)
    for i in 0..proof.trace_ood_frame_evaluations.num_rows() {
        for element in proof.trace_ood_frame_evaluations.get_row(i).iter() {
            transcript.append(&element.to_bytes_be());
        }
    }

    // ===================================
    // ==========|   Round 4   |==========
    // ===================================

    // >>>> Send challenges: 𝛾, 𝛾'
    let gamma_even = transcript_to_field(transcript);
    let gamma_odd = transcript_to_field(transcript);

    // >>>> Send challenges: 𝛾ⱼ, 𝛾ⱼ'
    // Get the number of trace terms the DEEP composition poly will have.
    // One coefficient will be sampled for each of them.
    // TODO: try remove this, call transcript inside for and move gamma declarations
    let trace_term_coeffs = (0..total_columns)
        .map(|_| {
            (0..air.context().transition_offsets.len())
                .map(|_| transcript_to_field(transcript))
                .collect()
        })
        .collect::<Vec<Vec<FieldElement<F>>>>();

    // FRI commit phase
    let mut zetas: Vec<FieldElement<F>> = Vec::new();
    let merkle_roots = &proof.fri_layers_merkle_roots;
    for root in merkle_roots.iter() {
        // <<<< Receive commitment: [pₖ] (the first one is [p₀])
        transcript.append(root);

        // >>>> Send challenge 𝜁ₖ
        let zeta = transcript_to_field(transcript);
        zetas.push(zeta);
    }

    // <<<< Receive value: pₙ
    transcript.append(&proof.fri_last_value.to_bytes_be());

    // FRI query phase
    // <<<< Send challenges 𝜄ₛ (iota_s)
    let iotas = (0..air.options().fri_number_of_queries)
        .map(|_| transcript_to_usize(transcript) % (2_usize.pow(domain.lde_root_order)))
        .collect();

    Challenges {
        z,
        boundary_coeffs,
        transition_coeffs,
        trace_term_coeffs,
        gamma_even,
        gamma_odd,
        zetas,
        iotas,
        rap_challenges,
    }
}

fn step_2_verify_claimed_composition_polynomial<F: IsFFTField, A: AIR<Field = F>>(
    air: &A,
    proof: &StarkProof<F>,
    domain: &Domain<F>,
    public_input: &A::PublicInput,
    challenges: &Challenges<F, A>,
) -> bool {
    // BEGIN TRACE <-> Composition poly consistency evaluation check
    // These are H_1(z^2) and H_2(z^2)
    let composition_poly_even_ood_evaluation = &proof.composition_poly_even_ood_evaluation;
    let composition_poly_odd_ood_evaluation = &proof.composition_poly_odd_ood_evaluation;

    let boundary_constraints = air.boundary_constraints(&challenges.rap_challenges, public_input);

    let n_trace_cols = air.context().trace_columns;

    let boundary_constraint_domains =
        boundary_constraints.generate_roots_of_unity(&domain.trace_primitive_root, n_trace_cols);
    let values = boundary_constraints.values(n_trace_cols);

    // Following naming conventions from https://www.notamonadtutorial.com/diving-deep-fri/
    let mut boundary_c_i_evaluations = Vec::with_capacity(n_trace_cols);
    let mut boundary_quotient_degrees = Vec::with_capacity(n_trace_cols);

    for trace_idx in 0..n_trace_cols {
        let trace_evaluation = &proof.trace_ood_frame_evaluations.get_row(0)[trace_idx];
        let boundary_constraints_domain = &boundary_constraint_domains[trace_idx];
        let boundary_interpolating_polynomial =
            &Polynomial::interpolate(boundary_constraints_domain, &values[trace_idx])
                .expect("xs and ys have equal length and xs are unique");

        let boundary_zerofier =
            boundary_constraints.compute_zerofier(&domain.trace_primitive_root, trace_idx);

        let boundary_quotient_ood_evaluation = (trace_evaluation
            - boundary_interpolating_polynomial.evaluate(&challenges.z))
            / boundary_zerofier.evaluate(&challenges.z);

        let boundary_quotient_degree = air.context().trace_length - boundary_zerofier.degree() - 1;

        boundary_c_i_evaluations.push(boundary_quotient_ood_evaluation);
        boundary_quotient_degrees.push(boundary_quotient_degree);
    }

    // TODO: Get trace polys degrees in a better way. The degree may not be trace_length - 1 in some
    // special cases.
    let trace_length = air.context().trace_length;

    let boundary_term_degree_adjustment = air.composition_poly_degree_bound() - trace_length;

    let boundary_quotient_ood_evaluations: Vec<FieldElement<F>> = boundary_c_i_evaluations
        .iter()
        .zip(&challenges.boundary_coeffs)
        .map(|(poly_eval, (alpha, beta))| {
            poly_eval * (alpha * challenges.z.pow(boundary_term_degree_adjustment) + beta)
        })
        .collect();

    let boundary_quotient_ood_evaluation = boundary_quotient_ood_evaluations
        .iter()
        .fold(FieldElement::<F>::zero(), |acc, x| acc + x);

    let transition_ood_frame_evaluations = air.compute_transition(
        &proof.trace_ood_frame_evaluations,
        &challenges.rap_challenges,
    );

    let transition_exemptions = air.transition_exemptions();

    let x_n = Polynomial::new_monomial(FieldElement::<F>::one(), trace_length);
    let x_n_1 = x_n - FieldElement::<F>::one();

    let divisors = transition_exemptions
        .into_iter()
        .map(|exemption| x_n_1.clone() / exemption)
        .collect::<Vec<Polynomial<FieldElement<F>>>>();

    let mut denominators = Vec::with_capacity(divisors.len());
    for divisor in divisors.iter() {
        denominators.push(divisor.evaluate(&challenges.z));
    }
    FieldElement::inplace_batch_inverse(&mut denominators);

    let mut degree_adjustments = Vec::with_capacity(divisors.len());
    for transition_degree in air.context().transition_degrees().iter() {
        let degree_adjustment = air.composition_poly_degree_bound()
            - (air.context().trace_length * (transition_degree - 1));
        degree_adjustments.push(challenges.z.pow(degree_adjustment));
    }
    let transition_c_i_evaluations_sum =
        ConstraintEvaluator::<F, A>::compute_constraint_composition_poly_evaluations_sum(
            &transition_ood_frame_evaluations,
            &denominators,
            &degree_adjustments,
            &challenges.transition_coeffs,
        );

    let composition_poly_ood_evaluation =
        &boundary_quotient_ood_evaluation + transition_c_i_evaluations_sum;

    let composition_poly_claimed_ood_evaluation =
        composition_poly_even_ood_evaluation + &challenges.z * composition_poly_odd_ood_evaluation;

    composition_poly_claimed_ood_evaluation == composition_poly_ood_evaluation
}

fn step_3_verify_fri<F, A>(
    air: &A,
    proof: &StarkProof<F>,
    domain: &Domain<F>,
    challenges: &Challenges<F, A>,
) -> bool
where
    F: IsFFTField,
    FieldElement<F>: ByteConversion,
    A: AIR<Field = F>,
{
    let mut result = true;
    // Verify FRI
    for (proof_s, iota_s) in proof.query_list.iter().zip(challenges.iotas.iter()) {
        // this is done in constant time
        result &= verify_query_and_sym_openings(
            air,
            &proof.fri_layers_merkle_roots,
            &proof.fri_last_value,
            &challenges.zetas,
            *iota_s,
            proof_s,
            domain,
        );
    }

    result
}

fn step_4_verify_deep_composition_polynomial<F: IsFFTField, A: AIR<Field = F>>(
    air: &A,
    proof: &StarkProof<F>,
    domain: &Domain<F>,
    challenges: &Challenges<F, A>,
) -> bool
where
    FieldElement<F>: ByteConversion,
{
    let mut result = true;

    let iota_0 = challenges.iotas[0];

    let evaluations = vec![
        proof
            .deep_poly_openings
            .lde_composition_poly_even_evaluation
            .clone(),
        proof
            .deep_poly_openings
            .lde_composition_poly_odd_evaluation
            .clone(),
    ];
    // Verify opening Open(H₁(D_LDE, 𝜐₀) and Open(H₂(D_LDE, 𝜐₀),
    result &= proof
        .deep_poly_openings
        .lde_composition_poly_proof
        .verify::<BatchStarkProverBackend<F>>(&proof.composition_poly_root, iota_0, &evaluations);

    let num_main_columns = air.context().trace_columns - air.number_auxiliary_rap_columns();
    let lde_trace_evaluations = vec![
        proof.deep_poly_openings.lde_trace_evaluations[..num_main_columns].to_vec(),
        proof.deep_poly_openings.lde_trace_evaluations[num_main_columns..].to_vec(),
    ];

    // Verify openings Open(tⱼ(D_LDE), 𝜐₀)
    for ((merkle_root, merkle_proof), evaluation) in proof
        .lde_trace_merkle_roots
        .iter()
        .zip(&proof.deep_poly_openings.lde_trace_merkle_proofs)
        .zip(lde_trace_evaluations)
    {
        result &=
            merkle_proof.verify::<BatchStarkProverBackend<F>>(merkle_root, iota_0, &evaluation);
    }

    // DEEP consistency check
    // Verify that Deep(x) is constructed correctly
    let deep_poly_evaluation =
        reconstruct_deep_composition_poly_evaluation(proof, domain, challenges);
    let deep_poly_claimed_evaluation = &proof.query_list[0].first_layer_evaluation;

    result &= deep_poly_claimed_evaluation == &deep_poly_evaluation;
    result
}

fn verify_query_and_sym_openings<F: IsField + IsFFTField, A: AIR<Field = F>>(
    air: &A,
    fri_layers_merkle_roots: &[Commitment],
    fri_last_value: &FieldElement<F>,
    zetas: &[FieldElement<F>],
    iota: usize,
    fri_decommitment: &FriDecommitment<F>,
    domain: &Domain<F>,
) -> bool
where
    FieldElement<F>: ByteConversion,
{
    // Verify opening Open(p₀(D₀), 𝜐ₛ)
    if !fri_decommitment
        .first_layer_auth_path
        .verify::<FriMerkleBackend<F>>(
            &fri_layers_merkle_roots[0],
            iota,
            &fri_decommitment.first_layer_evaluation,
        )
    {
        return false;
    }

    let lde_primitive_root = F::get_primitive_root_of_unity(domain.lde_root_order as u64).unwrap();
    let offset = FieldElement::from(air.options().coset_offset);
    // evaluation point = offset * w ^ i in the Stark literature
    let mut evaluation_point = offset * lde_primitive_root.pow(iota);

    let mut v = fri_decommitment.first_layer_evaluation.clone();
    // For each fri layer merkle proof check:
    // That each merkle path verifies

    // Sample beta with fiat shamir
    // Compute v = [P_i(z_i) + P_i(-z_i)] / 2 + beta * [P_i(z_i) - P_i(-z_i)] / (2 * z_i)
    // Where P_i is the folded polynomial of the i-th fiat shamir round
    // z_i is obtained from the first z (that was derived through Fiat-Shamir) through a known calculation
    // The calculation is, given the index, index % length_of_evaluation_domain

    // Check that v = P_{i+1}(z_i)

    // For each (merkle_root, merkle_auth_path) / fold
    // With the auth path containining the element that the path proves it's existence
    for (k, (merkle_root, (auth_path, evaluation_sym))) in fri_layers_merkle_roots
        .iter()
        .zip(
            fri_decommitment
                .layers_auth_paths_sym
                .iter()
                .zip(fri_decommitment.layers_evaluations_sym.iter()),
        )
        .enumerate()
    // Since we always derive the current layer from the previous layer
    // We start with the second one, skipping the first, so previous is layer is the first one
    {
        // This is the current layer's evaluation domain length.
        // We need it to know what the decommitment index for the current
        // layer is, so we can check the merkle paths at the right index.
        let domain_length = 1 << (domain.lde_root_order - k as u32);
        let layer_evaluation_index_sym = (iota + domain_length / 2) % domain_length;

        // Verify opening Open(pₖ(Dₖ), −𝜐ₛ^(2ᵏ))
        if !auth_path.verify::<FriMerkleBackend<F>>(
            merkle_root,
            layer_evaluation_index_sym,
            evaluation_sym,
        ) {
            return false;
        }

        let beta = &zetas[k];
        // v is the calculated element for the co linearity check
        let two = &FieldElement::from(2);
        v = (&v + evaluation_sym) / two + beta * (&v - evaluation_sym) / (two * &evaluation_point);
        evaluation_point = evaluation_point.pow(2_u64);
    }

    // Check that last value is the given by the prover
    v == *fri_last_value
}

// Reconstruct Deep(\upsilon_0) off the values in the proof
fn reconstruct_deep_composition_poly_evaluation<F: IsFFTField, A: AIR<Field = F>>(
    proof: &StarkProof<F>,
    domain: &Domain<F>,
    challenges: &Challenges<F, A>,
) -> FieldElement<F> {
    let primitive_root = &F::get_primitive_root_of_unity(domain.root_order as u64).unwrap();
    let upsilon_0 = &domain.lde_roots_of_unity_coset[challenges.iotas[0]];

    let mut trace_terms = FieldElement::zero();

    for (col_idx, coeff_row) in
        (0..proof.trace_ood_frame_evaluations.num_columns()).zip(&challenges.trace_term_coeffs)
    {
        for (row_idx, coeff) in (0..proof.trace_ood_frame_evaluations.num_rows()).zip(coeff_row) {
            let poly_evaluation = (proof.deep_poly_openings.lde_trace_evaluations[col_idx].clone()
                - proof.trace_ood_frame_evaluations.get_row(row_idx)[col_idx].clone())
                / (upsilon_0 - &challenges.z * primitive_root.pow(row_idx as u64));

            trace_terms += poly_evaluation * coeff.clone();
        }
    }

    let z_squared = &(&challenges.z * &challenges.z);
    let h_1_upsilon_0 = &proof
        .deep_poly_openings
        .lde_composition_poly_even_evaluation;
    let h_1_zsquared = &proof.composition_poly_even_ood_evaluation;
    let h_2_upsilon_0 = &proof.deep_poly_openings.lde_composition_poly_odd_evaluation;
    let h_2_zsquared = &proof.composition_poly_odd_ood_evaluation;

    let h_1_term = (h_1_upsilon_0 - h_1_zsquared) / (upsilon_0 - z_squared);
    let h_2_term = (h_2_upsilon_0 - h_2_zsquared) / (upsilon_0 - z_squared);

    trace_terms + h_1_term * &challenges.gamma_even + h_2_term * &challenges.gamma_odd
}

pub fn verify<F, A>(proof: &StarkProof<F>, air: &A, public_input: &A::PublicInput) -> bool
where
    F: IsFFTField,
    A: AIR<Field = F>,
    FieldElement<F>: ByteConversion,
{
    #[cfg(feature = "instruments")]
    println!("- Started step 1: Recover challenges");
    #[cfg(feature = "instruments")]
    let timer1 = Instant::now();

    let mut transcript = step_1_transcript_initialization();
    let domain = Domain::new(air);

    let challenges =
        step_1_replay_rounds_and_recover_challenges(air, proof, &domain, &mut transcript);

    #[cfg(feature = "instruments")]
    let elapsed1 = timer1.elapsed();
    #[cfg(feature = "instruments")]
    println!("  Time spent: {:?}", elapsed1);

    #[cfg(feature = "instruments")]
    println!("- Started step 2: Verify claimed polynomial");
    #[cfg(feature = "instruments")]
    let timer2 = Instant::now();

    if !step_2_verify_claimed_composition_polynomial(air, proof, &domain, public_input, &challenges)
    {
        error!("Composition Polynomial verification failed");
        return false;
    }

    #[cfg(feature = "instruments")]
    let elapsed2 = timer2.elapsed();
    #[cfg(feature = "instruments")]
    println!("  Time spent: {:?}", elapsed2);
    #[cfg(feature = "instruments")]

    println!("- Started step 3: Verify FRI");
    #[cfg(feature = "instruments")]
    let timer3 = Instant::now();

    if !step_3_verify_fri(air, proof, &domain, &challenges) {
        error!("FRI verification failed");
        return false;
    }

    #[cfg(feature = "instruments")]
    let elapsed3 = timer3.elapsed();
    #[cfg(feature = "instruments")]
    println!("  Time spent: {:?}", elapsed3);

    #[cfg(feature = "instruments")]
    println!("- Started step 4: Verify deep composition polynomial");
    #[cfg(feature = "instruments")]
    let timer4 = Instant::now();

    #[allow(clippy::let_and_return)]
    if !step_4_verify_deep_composition_polynomial(air, proof, &domain, &challenges) {
        error!("DEEP Composition Polynomial verification failed");
        return false;
    }

    #[cfg(feature = "instruments")]
    let elapsed4 = timer4.elapsed();
    #[cfg(feature = "instruments")]
    println!("  Time spent: {:?}", elapsed4);

    #[cfg(feature = "instruments")]
    {
        let total_time = elapsed1 + elapsed2 + elapsed3 + elapsed4;
        println!(
            " Fraction of verifying time per step: {:.4} {:.4} {:.4} {:.4}",
            elapsed1.as_nanos() as f64 / total_time.as_nanos() as f64,
            elapsed2.as_nanos() as f64 / total_time.as_nanos() as f64,
            elapsed3.as_nanos() as f64 / total_time.as_nanos() as f64,
            elapsed4.as_nanos() as f64 / total_time.as_nanos() as f64
        );
    }

    true
}
