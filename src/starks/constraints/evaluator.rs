use lambdaworks_math::{
    fft::cpu::roots_of_unity::get_powers_of_primitive_root_coset,
    field::{element::FieldElement, traits::IsFFTField},
    polynomial::Polynomial,
    traits::ByteConversion,
};

#[cfg(feature = "parallel")]
use rayon::prelude::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

#[cfg(all(debug_assertions, not(feature = "parallel")))]
use crate::starks::debug::check_boundary_polys_divisibility;
use crate::starks::domain::Domain;
use crate::starks::frame::Frame;
use crate::starks::prover::evaluate_polynomial_on_lde_domain;
use crate::starks::trace::TraceTable;
use crate::starks::traits::AIR;

use super::{boundary::BoundaryConstraints, evaluation_table::ConstraintEvaluationTable};

pub struct ConstraintEvaluator<'poly, F: IsFFTField, A: AIR> {
    air: A,
    boundary_constraints: BoundaryConstraints<F>,
    trace_polys: &'poly [Polynomial<FieldElement<F>>],
    primitive_root: FieldElement<F>,
}
impl<'poly, F: IsFFTField, A: AIR + AIR<Field = F>> ConstraintEvaluator<'poly, F, A> {
    pub fn new(
        air: &A,
        trace_polys: &'poly [Polynomial<FieldElement<F>>],
        primitive_root: &FieldElement<F>,
        rap_challenges: &A::RAPChallenges,
    ) -> Self {
        let boundary_constraints = air.boundary_constraints(rap_challenges);

        Self {
            air: air.clone(),
            boundary_constraints,
            trace_polys,
            primitive_root: primitive_root.clone(),
        }
    }

    pub fn evaluate(
        &self,
        lde_trace: &TraceTable<F>,
        domain: &Domain<F>,
        alpha_and_beta_transition_coefficients: &[(FieldElement<F>, FieldElement<F>)],
        alpha_and_beta_boundary_coefficients: &[(FieldElement<F>, FieldElement<F>)],
        rap_challenges: &A::RAPChallenges,
    ) -> ConstraintEvaluationTable<F>
    where
        FieldElement<F>: ByteConversion + Send + Sync,
        A: Send + Sync,
        A::RAPChallenges: Send + Sync,
    {
        // The + 1 is for the boundary constraints column
        let mut evaluation_table = ConstraintEvaluationTable::new(
            self.air.context().num_transition_constraints() + 1,
            &domain.lde_roots_of_unity_coset,
        );
        //let n_trace_colums = self.trace_polys.len();
        let boundary_constraints = &self.boundary_constraints;
        let number_of_b_constraints = boundary_constraints.constraints.len();
        let _boundary_steps = self.boundary_constraints.steps_for_boundary();
        let boundary_cols = self.boundary_constraints.cols_for_boundary();
        let boundary_zerofiers_inverse_evaluations: Vec<Vec<FieldElement<F>>> = (0
            ..number_of_b_constraints)
            .map(|step| {
                let point = &domain.trace_primitive_root.pow(step as u64);
                let mut evals = domain
                    .lde_roots_of_unity_coset
                    .iter()
                    .map(|v| v.clone() - point)
                    .collect::<Vec<FieldElement<F>>>();
                FieldElement::inplace_batch_inverse(&mut evals);
                evals
            })
            .collect::<Vec<Vec<FieldElement<F>>>>();

        let trace_length = self.air.trace_length();
        let composition_poly_degree_bound = self.air.composition_poly_degree_bound();
        let boundary_term_degree_adjustment = composition_poly_degree_bound - trace_length;
        // Maybe we can do this more efficiently by taking the offset's power and then using successors for roots of unity
        let d_adjustment_power = domain
            .lde_roots_of_unity_coset
            .iter()
            .map(|d| d.pow(boundary_term_degree_adjustment))
            .collect::<Vec<FieldElement<F>>>();

        let _domains =
            boundary_constraints.generate_roots_of_unity(&self.primitive_root, &boundary_cols);
        let _values = boundary_constraints.values(&boundary_cols);

        #[cfg(all(debug_assertions, not(feature = "parallel")))]
        let boundary_polys: Vec<Polynomial<FieldElement<F>>> = Vec::new();

        #[cfg(not(feature = "parallel"))]
        let _boundary_iter = boundary_cols.iter();

        #[cfg(feature = "parallel")]
        let boundary_iter = boundary_cols.par_iter();

        let boundary_polys_evaluations = boundary_constraints
            .constraints
            .iter()
            .map(|constraint| {
                let col = constraint.col;
                let boundary_poly = &self.trace_polys[col] - &constraint.value;

                evaluate_polynomial_on_lde_domain(
                    &boundary_poly,
                    domain.blowup_factor,
                    domain.interpolation_domain_size,
                    &domain.coset_offset,
                )
                .unwrap()
            })
            .collect::<Vec<Vec<FieldElement<F>>>>();
        let boundary_evaluation = (0..domain.lde_roots_of_unity_coset.len())
            .map(|i| {
                (0..number_of_b_constraints).fold(FieldElement::zero(), |acc, index| {
                    let (alpha, beta) = &alpha_and_beta_boundary_coefficients[index];
                    acc + &boundary_zerofiers_inverse_evaluations[index][i]
                        * (alpha * &d_adjustment_power[i] + beta)
                        * &boundary_polys_evaluations[index][i]
                })
            })
            .collect::<Vec<FieldElement<F>>>();

        #[cfg(all(debug_assertions, not(feature = "parallel")))]
        let boundary_zerofiers = Vec::new();

        #[cfg(all(debug_assertions, not(feature = "parallel")))]
        check_boundary_polys_divisibility(boundary_polys, boundary_zerofiers);

        let blowup_factor = self.air.blowup_factor();

        #[cfg(all(debug_assertions, not(feature = "parallel")))]
        let mut transition_evaluations = Vec::new();

        let transition_exemptions = self.air.transition_exemptions();

        let transition_exemptions_evaluations =
            evaluate_transition_exemptions(transition_exemptions, domain);

        let context = self.air.context();
        let max_transition_degree = *context.transition_degrees.iter().max().unwrap();

        #[cfg(feature = "parallel")]
        let degree_adjustments_iter = (1..=max_transition_degree).into_par_iter();

        #[cfg(not(feature = "parallel"))]
        let degree_adjustments_iter = 1..=max_transition_degree;

        let degree_adjustments: Vec<Vec<FieldElement<F>>> = degree_adjustments_iter
            .map(|transition_degree| {
                domain
                    .lde_roots_of_unity_coset
                    .iter()
                    .map(|d| {
                        let degree_adjustment = composition_poly_degree_bound
                            - (trace_length * (transition_degree - 1));
                        d.pow(degree_adjustment)
                    })
                    .collect()
            })
            .collect();

        let blowup_factor_order = u64::from(blowup_factor.trailing_zeros());

        let offset = FieldElement::<F>::from(self.air.context().proof_options.coset_offset);
        let offset_pow = offset.pow(trace_length);
        let one = FieldElement::<F>::one();
        let mut zerofier_evaluations = get_powers_of_primitive_root_coset(
            blowup_factor_order,
            blowup_factor as usize,
            &offset_pow,
        )
        .unwrap()
        .iter()
        .map(|v| v - &one)
        .collect::<Vec<_>>();

        FieldElement::inplace_batch_inverse(&mut zerofier_evaluations);
        let transition_zerofiers_inverse_evaluations: Vec<Vec<FieldElement<F>>> =
            transition_exemptions_evaluations
                .iter()
                .map(|row| {
                    zerofier_evaluations
                        .iter()
                        .cycle()
                        .zip(row)
                        .map(|(c1, c2)| c1 * c2)
                        .collect()
                })
                .collect();

        // Iterate over trace and domain and compute transitions
        #[cfg(feature = "parallel")]
        let evaluations_t_iter = (0..domain.lde_roots_of_unity_coset.len()).into_par_iter();

        #[cfg(not(feature = "parallel"))]
        let evaluations_t_iter = 0..domain.lde_roots_of_unity_coset.len();

        let evaluations_t = evaluations_t_iter
            .map(|i| {
                let frame = Frame::read_from_trace(
                    lde_trace,
                    i,
                    blowup_factor,
                    &self.air.context().transition_offsets,
                );

                let evaluations_transition = self.air.compute_transition(&frame, rap_challenges);

                #[cfg(all(debug_assertions, not(feature = "parallel")))]
                transition_evaluations.push(evaluations_transition.clone());

                // TODO: Remove clones
                let denominators: Vec<_> = transition_zerofiers_inverse_evaluations
                    .iter()
                    .map(|zerofier_evals| zerofier_evals[i].clone())
                    .collect();
                let degree_adjustments: Vec<_> = context
                    .transition_degrees
                    .iter()
                    .map(|&transition_adjustments| {
                        degree_adjustments[transition_adjustments - 1][i].clone()
                    })
                    .collect();

                let mut evaluations_sum = Self::compute_constraint_composition_poly_evaluations_sum(
                    &evaluations_transition,
                    &denominators,
                    &degree_adjustments,
                    alpha_and_beta_transition_coefficients,
                );

                evaluations_sum += boundary_evaluation[i].clone();

                evaluations_sum
            })
            .collect::<Vec<FieldElement<F>>>();

        evaluation_table.evaluations_acc = evaluations_t;

        evaluation_table
    }

    /// Given `evaluations` T_i(x) of the trace polynomial composed with the constraint
    /// polynomial at a certain point `x`, computes the following evaluations and returns them:
    ///
    /// T_i(x) (alpha_i * x^(D - D_i) + beta_i) / (Z_i(x))
    ///
    /// where Z is the zerofier of the `i`-th transition constraint polynomial. In the fibonacci
    /// example, T_i(x) is t(x * g^2) - t(x * g) - t(x).
    ///
    /// This method is called in two different scenarios. The first is when the prover
    /// is building these evaluations to compute the composition and DEEP composition polynomials.
    /// The second one is when the verifier needs to check the consistency between the trace and
    /// the composition polynomial. In that case the `evaluations` are over an *out of domain* frame
    /// (in the fibonacci example they are evaluations on the points `z`, `zg`, `zg^2`).
    ///
    /// # Returns
    ///
    /// Returns the sum of the evaluations computed.
    pub fn compute_constraint_composition_poly_evaluations_sum(
        evaluations: &[FieldElement<F>],
        inverse_denominators: &[FieldElement<F>],
        degree_adjustments: &[FieldElement<F>],
        constraint_coeffs: &[(FieldElement<F>, FieldElement<F>)],
    ) -> FieldElement<F> {
        evaluations
            .iter()
            .zip(degree_adjustments)
            .zip(inverse_denominators)
            .zip(constraint_coeffs)
            .fold(
                FieldElement::<F>::zero(),
                |acc, (((ev, degree), inv), coeff)| {
                    let (alpha, beta) = &coeff;
                    acc + ev * (alpha * degree + beta) * inv
                },
            )
    }
}

fn evaluate_transition_exemptions<F: IsFFTField>(
    transition_exemptions: Vec<Polynomial<FieldElement<F>>>,
    domain: &Domain<F>,
) -> Vec<Vec<FieldElement<F>>>
where
    FieldElement<F>: Send + Sync,
    Polynomial<FieldElement<F>>: Send + Sync,
{
    #[cfg(feature = "parallel")]
    let exemptions_iter = transition_exemptions.par_iter();
    #[cfg(not(feature = "parallel"))]
    let exemptions_iter = transition_exemptions.iter();

    exemptions_iter
        .map(|exemption| {
            evaluate_polynomial_on_lde_domain(
                exemption,
                domain.blowup_factor,
                domain.interpolation_domain_size,
                &domain.coset_offset,
            )
            .unwrap()
        })
        .collect()
}
