use std::{collections::HashMap, ops::Range};

use lambdaworks_crypto::fiat_shamir::transcript::Transcript;
use lambdaworks_math::field::{
    element::FieldElement, fields::fft_friendly::stark_252_prime_field::Stark252PrimeField,
};

use crate::{
    starks::{
        constraints::boundary::{BoundaryConstraint, BoundaryConstraints},
        context::AirContext,
        frame::Frame,
        proof_options::ProofOptions,
        trace::TraceTable,
        traits::AIR,
        transcript::transcript_to_field,
    },
    FE,
};

use super::{cairo_mem::CairoMemory, register_states::RegisterStates};

/// Main constraint identifiers
const INST: usize = 16;
const DST_ADDR: usize = 17;
const OP0_ADDR: usize = 18;
const OP1_ADDR: usize = 19;
const NEXT_AP: usize = 20;
const NEXT_FP: usize = 21;
const NEXT_PC_1: usize = 22;
const NEXT_PC_2: usize = 23;
const T0: usize = 24;
const T1: usize = 25;
const MUL_1: usize = 26;
const MUL_2: usize = 27;
const CALL_1: usize = 28;
const CALL_2: usize = 29;
const ASSERT_EQ: usize = 30;

// Auxiliary constraint identifiers
const MEMORY_INCREASING_0: usize = 31;
const MEMORY_INCREASING_1: usize = 32;
const MEMORY_INCREASING_2: usize = 33;
const MEMORY_INCREASING_3: usize = 34;

const MEMORY_CONSISTENCY_0: usize = 35;
const MEMORY_CONSISTENCY_1: usize = 36;
const MEMORY_CONSISTENCY_2: usize = 37;
const MEMORY_CONSISTENCY_3: usize = 38;

const PERMUTATION_ARGUMENT_0: usize = 39;
const PERMUTATION_ARGUMENT_1: usize = 40;
const PERMUTATION_ARGUMENT_2: usize = 41;
const PERMUTATION_ARGUMENT_3: usize = 42;

const RANGE_CHECK_INCREASING_0: usize = 43;
const RANGE_CHECK_INCREASING_1: usize = 44;
const RANGE_CHECK_INCREASING_2: usize = 45;

const RANGE_CHECK_0: usize = 46;
const RANGE_CHECK_1: usize = 47;
const RANGE_CHECK_2: usize = 48;

// Range-check builtin value decomposition constraint
const RANGE_CHECK_BUILTIN: usize = 49;

// Frame row identifiers
//  - Flags
const F_DST_FP: usize = 0;
const F_OP_0_FP: usize = 1;
const F_OP_1_VAL: usize = 2;
const F_OP_1_FP: usize = 3;
const F_OP_1_AP: usize = 4;
const F_RES_ADD: usize = 5;
const F_RES_MUL: usize = 6;
const F_PC_ABS: usize = 7;
const F_PC_REL: usize = 8;
const F_PC_JNZ: usize = 9;
const F_AP_ADD: usize = 10;
const F_AP_ONE: usize = 11;
const F_OPC_CALL: usize = 12;
const F_OPC_RET: usize = 13;
const F_OPC_AEQ: usize = 14;

//  - Others
// TODO: These should probably be in the TraceTable module.
pub const FRAME_RES: usize = 16;
pub const FRAME_AP: usize = 17;
pub const FRAME_FP: usize = 18;
pub const FRAME_PC: usize = 19;
pub const FRAME_DST_ADDR: usize = 20;
pub const FRAME_OP0_ADDR: usize = 21;
pub const FRAME_OP1_ADDR: usize = 22;
pub const FRAME_INST: usize = 23;
pub const FRAME_DST: usize = 24;
pub const FRAME_OP0: usize = 25;
pub const FRAME_OP1: usize = 26;
pub const OFF_DST: usize = 27;
pub const OFF_OP0: usize = 28;
pub const OFF_OP1: usize = 29;
pub const FRAME_T0: usize = 30;
pub const FRAME_T1: usize = 31;
pub const FRAME_MUL: usize = 32;
pub const FRAME_SELECTOR: usize = 33;

// Range-check frame identifiers
pub const RC_0: usize = 34;
pub const RC_1: usize = 35;
pub const RC_2: usize = 36;
pub const RC_3: usize = 37;
pub const RC_4: usize = 38;
pub const RC_5: usize = 39;
pub const RC_6: usize = 40;
pub const RC_7: usize = 41;
pub const RC_VALUE: usize = 42;

// Auxiliary range check columns
pub const RANGE_CHECK_COL_1: usize = 43;
pub const RANGE_CHECK_COL_2: usize = 44;
pub const RANGE_CHECK_COL_3: usize = 45;

// Auxiliary memory columns
pub const MEMORY_ADDR_SORTED_0: usize = 46;
pub const MEMORY_ADDR_SORTED_1: usize = 47;
pub const MEMORY_ADDR_SORTED_2: usize = 48;
pub const MEMORY_ADDR_SORTED_3: usize = 49;

pub const MEMORY_VALUES_SORTED_0: usize = 50;
pub const MEMORY_VALUES_SORTED_1: usize = 51;
pub const MEMORY_VALUES_SORTED_2: usize = 52;
pub const MEMORY_VALUES_SORTED_3: usize = 53;

pub const PERMUTATION_ARGUMENT_COL_0: usize = 54;
pub const PERMUTATION_ARGUMENT_COL_1: usize = 55;
pub const PERMUTATION_ARGUMENT_COL_2: usize = 56;
pub const PERMUTATION_ARGUMENT_COL_3: usize = 57;

pub const PERMUTATION_ARGUMENT_RANGE_CHECK_COL_1: usize = 58;
pub const PERMUTATION_ARGUMENT_RANGE_CHECK_COL_2: usize = 59;
pub const PERMUTATION_ARGUMENT_RANGE_CHECK_COL_3: usize = 60;

// Trace layout
pub const MEM_P_TRACE_OFFSET: usize = 17;
pub const MEM_A_TRACE_OFFSET: usize = 19;

// If Cairo AIR doesn't implement builtins, the auxiliary columns should have a smaller
// index.
const BUILTIN_OFFSET: usize = 9;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MemorySegment {
    RangeCheck,
    Output,
}

pub type MemorySegmentMap = HashMap<MemorySegment, Range<u64>>;

// TODO: For memory constraints and builtins, the commented fields may be useful.
#[derive(Clone)]
pub struct PublicInputs {
    pub pc_init: FE,
    pub ap_init: FE,
    pub fp_init: FE,
    pub pc_final: FE,
    pub ap_final: FE,
    // These are Option because they're not known until
    // the trace is obtained. They represent the minimum
    // and maximum offsets used during program execution.
    // TODO: A possible refactor is moving them to the proof.
    // minimum range check value (0 < range_check_min < range_check_max < 2^16)
    pub range_check_min: Option<u16>,
    // maximum range check value
    pub range_check_max: Option<u16>,
    // Range-check builtin address range
    pub memory_segments: MemorySegmentMap,
    pub public_memory: HashMap<FE, FE>,
    pub num_steps: usize, // number of execution steps
}

impl PublicInputs {
    /// Creates a Public Input from register states and memory
    /// - In the future we should use the output of the Cairo Runner. This is not currently supported in Cairo RS
    /// - RangeChecks are not filled, and the prover mutates them inside the prove function. This works but also should be loaded from the Cairo RS output
    pub fn from_regs_and_mem(
        register_states: &RegisterStates,
        memory: &CairoMemory,
        program_size: usize,
        memory_segments: &MemorySegmentMap,
    ) -> Self {
        let output_range = memory_segments.get(&MemorySegment::Output);

        let public_memory_size = if let Some(output_range) = output_range {
            program_size + (output_range.end - output_range.start) as usize
        } else {
            program_size
        };
        let mut public_memory = HashMap::with_capacity(public_memory_size);

        for i in 1..=program_size as u64 {
            public_memory.insert(FE::from(i), memory.get(&i).unwrap().clone());
        }

        if let Some(output_range) = output_range {
            for addr in output_range.clone() {
                public_memory.insert(FE::from(addr), memory.get(&addr).unwrap().clone());
            }
        };
        let last_step = &register_states.rows[register_states.steps() - 1];

        PublicInputs {
            pc_init: FE::from(register_states.rows[0].pc),
            ap_init: FE::from(register_states.rows[0].ap),
            fp_init: FE::from(register_states.rows[0].fp),
            pc_final: FieldElement::from(last_step.pc),
            ap_final: FieldElement::from(last_step.ap),
            range_check_min: None,
            range_check_max: None,
            memory_segments: memory_segments.clone(),
            public_memory,
            num_steps: register_states.steps(),
        }
    }
}
#[derive(Clone)]
pub struct CairoAIR {
    pub context: AirContext,
    pub number_steps: usize,
    has_rc_builtin: bool,
}

impl CairoAIR {
    /// Creates a new CairoAIR from proof_options
    ///
    /// # Arguments
    ///
    /// * `full_trace_length` - Trace length padded to 2^n
    /// * `number_steps` - Number of steps of the execution / register steps / rows in cairo runner trace
    /// * `has_rc_builtin` - `true` if the related program uses the range-check builtin, `false` otherwise
    #[rustfmt::skip]
    pub fn new(proof_options: ProofOptions, full_trace_length: usize, number_steps: usize, has_rc_builtin: bool) -> Self {
        let mut trace_columns = 34 + 3 + 12 + 3;
        let mut transition_degrees = vec![
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // Flags 0-14.
            1, // Flag 15
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // Other constraints.
            2, 2, 2, 2, // Increasing memory auxiliary constraints.
            2, 2, 2, 2, // Consistent memory auxiliary constraints.
            2, 2, 2, 2, // Permutation auxiliary constraints.
            2, 2, 2, // range-check increasing constraints.
            2, 2, 2, // range-check permutation argument constraints.
        ];
        let mut transition_exemptions = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // flags (16)
            0, // inst (1)
            0, 0, 0, // operand consraints (3)
            1, 1, 1, 1, 0, 0, // register constraints (6)
            0, 0, 0, 0, 0, // opcode constraints (5)
            0, 0, 0, 1, // memory continuous (4)
            0, 0, 0, 1, // memory value consistency (4)
            0, 0, 0, 1, // memory permutation argument (4)
            0, 0, 1, // range check continuous (3)
            0, 0, 0, // range check permutation argument (3)
        ];
        let mut num_transition_constraints = 49;

        if has_rc_builtin {
            trace_columns += 8 + 1; // 8 columns for each rc of the range-check builtin values decomposition, 1 for the values
            transition_degrees.push(1); // Range check builtin constraint
            transition_exemptions.push(0); // range-check builtin exemption
            num_transition_constraints += 1; // range-check builtin value decomposition constraint
        }

        let context = AirContext {
            options: proof_options,
            trace_length: full_trace_length,
            trace_columns,
            transition_degrees,
            transition_exemptions,
            transition_offsets: vec![0, 1],
            num_transition_constraints,
        };

        // The number of the transition constraints and the lengths of transition degrees
        // and transition exemptions should be the same always.
        debug_assert_eq!(
            context.transition_degrees.len(),
            context.num_transition_constraints
        );
        debug_assert_eq!(
            context.transition_exemptions.len(),
            context.num_transition_constraints
        );

        Self {
            context,
            number_steps,
            has_rc_builtin,
        }
    }

    fn get_builtin_offset(&self) -> usize {
        if self.has_rc_builtin {
            0
        } else {
            BUILTIN_OFFSET
        }
    }
}

pub struct CairoRAPChallenges {
    pub alpha_memory: FieldElement<Stark252PrimeField>,
    pub z_memory: FieldElement<Stark252PrimeField>,
    pub z_range_check: FieldElement<Stark252PrimeField>,
}

fn add_pub_memory_in_public_input_section(
    addresses: &Vec<FE>,
    values: &[FE],
    public_input: &PublicInputs,
) -> (Vec<FE>, Vec<FE>) {
    let mut a_aux = addresses.clone();
    let mut v_aux = values.to_owned();

    let public_input_section = addresses.len() - public_input.public_memory.len();
    let output_range = public_input.memory_segments.get(&MemorySegment::Output);
    let pub_memory_addrs = get_pub_memory_addrs(output_range, public_input);

    a_aux.splice(public_input_section.., pub_memory_addrs);
    for i in public_input_section..a_aux.len() {
        let address = &a_aux[i];
        v_aux[i] = public_input.public_memory.get(address).unwrap().clone();
    }

    (a_aux, v_aux)
}

/// Gets public memory addresses of a program. First, this function builds a `Vec` of `FieldElement`s, filling it
/// incrementally with addresses from `1` to `program_len - 1`, where `program_len` is the length of the program.
/// If the output builtin is used, `output_range` is `Some(...)` and this function adds incrementally to the resulting
/// `Vec` addresses from the start to the end of the unwrapped `output_range`.
fn get_pub_memory_addrs(
    output_range: Option<&Range<u64>>,
    public_input: &PublicInputs,
) -> Vec<FieldElement<Stark252PrimeField>> {
    let public_memory_len = public_input.public_memory.len() as u64;

    if let Some(output_range) = output_range {
        let output_section = output_range.end - output_range.start;
        let program_section = public_memory_len - output_section;

        (1..=program_section)
            .map(FieldElement::from)
            .chain(output_range.clone().map(FieldElement::from))
            .collect()
    } else {
        (1..=public_memory_len).map(FieldElement::from).collect()
    }
}

fn sort_columns_by_memory_address(adresses: Vec<FE>, values: Vec<FE>) -> (Vec<FE>, Vec<FE>) {
    let mut tuples: Vec<_> = adresses.into_iter().zip(values).collect();
    tuples.sort_by(|(x, _), (y, _)| x.representative().cmp(&y.representative()));
    tuples.into_iter().unzip()
}

fn generate_memory_permutation_argument_column(
    addresses_original: Vec<FE>,
    values_original: Vec<FE>,
    addresses_sorted: &[FE],
    values_sorted: &[FE],
    rap_challenges: &CairoRAPChallenges,
) -> Vec<FE> {
    let z = &rap_challenges.z_memory;
    let alpha = &rap_challenges.alpha_memory;
    let f = |a, v, ap, vp| (z - (a + alpha * v)) / (z - (ap + alpha * vp));

    let mut permutation_col = Vec::with_capacity(addresses_sorted.len());
    permutation_col.push(f(
        &addresses_original[0],
        &values_original[0],
        &addresses_sorted[0],
        &values_sorted[0],
    ));

    for i in 1..addresses_sorted.len() {
        let last = permutation_col.last().unwrap();
        permutation_col.push(
            last * f(
                &addresses_original[i],
                &values_original[i],
                &addresses_sorted[i],
                &values_sorted[i],
            ),
        );
    }

    permutation_col
}
fn generate_range_check_permutation_argument_column(
    offset_column_original: &[FE],
    offset_column_sorted: &[FE],
    rap_challenges: &CairoRAPChallenges,
) -> Vec<FE> {
    let z = &rap_challenges.z_range_check;
    let f = |a, ap| (z - a) / (z - ap);

    let mut permutation_col = Vec::with_capacity(offset_column_original.len());
    permutation_col.push(f(&offset_column_original[0], &offset_column_sorted[0]));

    for i in 1..offset_column_sorted.len() {
        let last = permutation_col.last().unwrap();
        permutation_col.push(last * f(&offset_column_original[i], &offset_column_sorted[i]));
    }
    permutation_col
}

impl AIR for CairoAIR {
    type Field = Stark252PrimeField;
    type RAPChallenges = CairoRAPChallenges;
    type PublicInput = PublicInputs;

    fn build_auxiliary_trace(
        &self,
        main_trace: &TraceTable<Self::Field>,
        rap_challenges: &Self::RAPChallenges,
        public_input: &Self::PublicInput,
    ) -> TraceTable<Self::Field> {
        let addresses_original = main_trace
            .get_cols(&[FRAME_PC, FRAME_DST_ADDR, FRAME_OP0_ADDR, FRAME_OP1_ADDR])
            .table;
        let values_original = main_trace
            .get_cols(&[FRAME_INST, FRAME_DST, FRAME_OP0, FRAME_OP1])
            .table;

        let (addresses, values) = add_pub_memory_in_public_input_section(
            &addresses_original,
            &values_original,
            public_input,
        );
        let (addresses, values) = sort_columns_by_memory_address(addresses, values);

        let permutation_col = generate_memory_permutation_argument_column(
            addresses_original,
            values_original,
            &addresses,
            &values,
            rap_challenges,
        );

        // Range Check
        let offsets_original = main_trace.get_cols(&[OFF_DST, OFF_OP0, OFF_OP1]).table;
        let mut offsets_sorted: Vec<u16> = offsets_original
            .iter()
            .map(|x| x.representative().into())
            .collect();
        offsets_sorted.sort();
        let offsets_sorted: Vec<_> = offsets_sorted
            .iter()
            .map(|x| FieldElement::from(*x as u64))
            .collect();

        let range_check_permutation_col = generate_range_check_permutation_argument_column(
            &offsets_original,
            &offsets_sorted,
            rap_challenges,
        );

        // Convert from long-format to wide-format again
        let mut aux_table = Vec::new();
        for i in 0..main_trace.n_rows() {
            aux_table.push(offsets_sorted[3 * i].clone());
            aux_table.push(offsets_sorted[3 * i + 1].clone());
            aux_table.push(offsets_sorted[3 * i + 2].clone());
            aux_table.push(addresses[4 * i].clone());
            aux_table.push(addresses[4 * i + 1].clone());
            aux_table.push(addresses[4 * i + 2].clone());
            aux_table.push(addresses[4 * i + 3].clone());
            aux_table.push(values[4 * i].clone());
            aux_table.push(values[4 * i + 1].clone());
            aux_table.push(values[4 * i + 2].clone());
            aux_table.push(values[4 * i + 3].clone());
            aux_table.push(permutation_col[4 * i].clone());
            aux_table.push(permutation_col[4 * i + 1].clone());
            aux_table.push(permutation_col[4 * i + 2].clone());
            aux_table.push(permutation_col[4 * i + 3].clone());
            aux_table.push(range_check_permutation_col[3 * i].clone());
            aux_table.push(range_check_permutation_col[3 * i + 1].clone());
            aux_table.push(range_check_permutation_col[3 * i + 2].clone());
        }

        TraceTable::new(aux_table, self.number_auxiliary_rap_columns())
    }

    fn build_rap_challenges<T: Transcript>(&self, transcript: &mut T) -> Self::RAPChallenges {
        CairoRAPChallenges {
            alpha_memory: transcript_to_field(transcript),
            z_memory: transcript_to_field(transcript),
            z_range_check: transcript_to_field(transcript),
        }
    }

    fn number_auxiliary_rap_columns(&self) -> usize {
        12 + 3 + 3
    }

    fn compute_transition(
        &self,
        frame: &Frame<Self::Field>,
        rap_challenges: &Self::RAPChallenges,
    ) -> Vec<FieldElement<Self::Field>> {
        let builtin_offset = self.get_builtin_offset();

        let mut constraints: Vec<FieldElement<Self::Field>> =
            vec![FE::zero(); self.num_transition_constraints()];

        compute_instr_constraints(&mut constraints, frame);
        compute_operand_constraints(&mut constraints, frame);
        compute_register_constraints(&mut constraints, frame);
        compute_opcode_constraints(&mut constraints, frame);
        enforce_selector(&mut constraints, frame);
        memory_is_increasing(&mut constraints, frame, builtin_offset);
        permutation_argument(&mut constraints, frame, rap_challenges, builtin_offset);
        permutation_argument_range_check(&mut constraints, frame, rap_challenges, builtin_offset);

        if self.has_rc_builtin {
            range_check_builtin(&mut constraints, frame);
        }

        constraints
    }

    /// From the Cairo whitepaper, section 9.10.
    /// These are part of the register constraints.
    ///
    /// Boundary constraints:
    ///  * ap_0 = fp_0 = ap_i
    ///  * ap_t = ap_f
    ///  * pc_0 = pc_i
    ///  * pc_t = pc_f
    fn boundary_constraints(
        &self,
        rap_challenges: &Self::RAPChallenges,
        public_input: &Self::PublicInput,
    ) -> BoundaryConstraints<Self::Field> {
        let initial_pc =
            BoundaryConstraint::new(MEM_A_TRACE_OFFSET, 0, public_input.pc_init.clone());
        let initial_ap =
            BoundaryConstraint::new(MEM_P_TRACE_OFFSET, 0, public_input.ap_init.clone());

        let final_pc = BoundaryConstraint::new(
            MEM_A_TRACE_OFFSET,
            self.number_steps - 1,
            public_input.pc_final.clone(),
        );
        let final_ap = BoundaryConstraint::new(
            MEM_P_TRACE_OFFSET,
            self.number_steps - 1,
            public_input.ap_final.clone(),
        );

        // Auxiliary constraint: permutation argument final value
        let final_index = self.context.trace_length - 1;

        let builtin_offset = self.get_builtin_offset();

        let mut cumulative_product = FieldElement::one();
        for (address, value) in &public_input.public_memory {
            cumulative_product = cumulative_product
                * (&rap_challenges.z_memory - (address + &rap_challenges.alpha_memory * value));
        }
        let permutation_final = rap_challenges
            .z_memory
            .pow(public_input.public_memory.len())
            / cumulative_product;
        let permutation_final_constraint = BoundaryConstraint::new(
            PERMUTATION_ARGUMENT_COL_3 - builtin_offset,
            final_index,
            permutation_final,
        );

        let one: FieldElement<Self::Field> = FieldElement::one();
        let range_check_final_constraint = BoundaryConstraint::new(
            PERMUTATION_ARGUMENT_RANGE_CHECK_COL_3 - builtin_offset,
            final_index,
            one,
        );

        let range_check_min = BoundaryConstraint::new(
            RANGE_CHECK_COL_1 - builtin_offset,
            0,
            FieldElement::from(public_input.range_check_min.unwrap() as u64),
        );
        let range_check_max = BoundaryConstraint::new(
            RANGE_CHECK_COL_3 - builtin_offset,
            final_index,
            FieldElement::from(public_input.range_check_max.unwrap() as u64),
        );

        let constraints = vec![
            initial_pc,
            initial_ap,
            final_pc,
            final_ap,
            permutation_final_constraint,
            range_check_final_constraint,
            range_check_min,
            range_check_max,
        ];

        BoundaryConstraints::from_constraints(constraints)
    }

    fn context(&self) -> &AirContext {
        &self.context
    }

    fn composition_poly_degree_bound(&self) -> usize {
        2 * self.context().trace_length
    }
}

/// From the Cairo whitepaper, section 9.10
fn compute_instr_constraints(constraints: &mut [FE], frame: &Frame<Stark252PrimeField>) {
    // These constraints are only applied over elements of the same row.
    let curr = frame.get_row(0);

    // Bit constraints
    for (i, flag) in curr[0..16].iter().enumerate() {
        constraints[i] = match i {
            0..=14 => flag * (flag - FE::one()),
            15 => flag.clone(),
            _ => panic!("Unknown flag offset"),
        };
    }

    // Instruction unpacking
    let two = FE::from(2);
    let b16 = two.pow(16u32);
    let b32 = two.pow(32u32);
    let b48 = two.pow(48u32);

    // Named like this to match the Cairo whitepaper's notation.
    let f0_squiggle = &curr[0..15]
        .iter()
        .rev()
        .fold(FE::zero(), |acc, flag| flag + &two * acc);

    constraints[INST] =
        (&curr[OFF_DST]) + b16 * (&curr[OFF_OP0]) + b32 * (&curr[OFF_OP1]) + b48 * f0_squiggle
            - &curr[FRAME_INST];
}

fn compute_operand_constraints(constraints: &mut [FE], frame: &Frame<Stark252PrimeField>) {
    // These constraints are only applied over elements of the same row.
    let curr = frame.get_row(0);

    let ap = &curr[FRAME_AP];
    let fp = &curr[FRAME_FP];
    let pc = &curr[FRAME_PC];

    let one = FE::one();
    let b15 = FE::from(2).pow(15u32);

    constraints[DST_ADDR] =
        &curr[F_DST_FP] * fp + (&one - &curr[F_DST_FP]) * ap + (&curr[OFF_DST] - &b15)
            - &curr[FRAME_DST_ADDR];

    constraints[OP0_ADDR] =
        &curr[F_OP_0_FP] * fp + (&one - &curr[F_OP_0_FP]) * ap + (&curr[OFF_OP0] - &b15)
            - &curr[FRAME_OP0_ADDR];

    constraints[OP1_ADDR] = &curr[F_OP_1_VAL] * pc
        + &curr[F_OP_1_AP] * ap
        + &curr[F_OP_1_FP] * fp
        + (&one - &curr[F_OP_1_VAL] - &curr[F_OP_1_AP] - &curr[F_OP_1_FP]) * &curr[FRAME_OP0]
        + (&curr[OFF_OP1] - &b15)
        - &curr[FRAME_OP1_ADDR];
}

fn compute_register_constraints(constraints: &mut [FE], frame: &Frame<Stark252PrimeField>) {
    let curr = frame.get_row(0);
    let next = frame.get_row(1);

    let one = FE::one();
    let two = FE::from(2);

    // ap and fp constraints
    constraints[NEXT_AP] = &curr[FRAME_AP]
        + &curr[F_AP_ADD] * &curr[FRAME_RES]
        + &curr[F_AP_ONE]
        + &curr[F_OPC_CALL] * &two
        - &next[FRAME_AP];

    constraints[NEXT_FP] = &curr[F_OPC_RET] * &curr[FRAME_DST]
        + &curr[F_OPC_CALL] * (&curr[FRAME_AP] + &two)
        + (&one - &curr[F_OPC_RET] - &curr[F_OPC_CALL]) * &curr[FRAME_FP]
        - &next[FRAME_FP];

    // pc constraints
    constraints[NEXT_PC_1] = (&curr[FRAME_T1] - &curr[F_PC_JNZ])
        * (&next[FRAME_PC] - (&curr[FRAME_PC] + frame_inst_size(curr)));

    constraints[NEXT_PC_2] = &curr[FRAME_T0]
        * (&next[FRAME_PC] - (&curr[FRAME_PC] + &curr[FRAME_OP1]))
        + (&one - &curr[F_PC_JNZ]) * &next[FRAME_PC]
        - ((&one - &curr[F_PC_ABS] - &curr[F_PC_REL] - &curr[F_PC_JNZ])
            * (&curr[FRAME_PC] + frame_inst_size(curr))
            + &curr[F_PC_ABS] * &curr[FRAME_RES]
            + &curr[F_PC_REL] * (&curr[FRAME_PC] + &curr[FRAME_RES]));

    constraints[T0] = &curr[F_PC_JNZ] * &curr[FRAME_DST] - &curr[FRAME_T0];
    constraints[T1] = &curr[FRAME_T0] * &curr[FRAME_RES] - &curr[FRAME_T1];
}

fn compute_opcode_constraints(constraints: &mut [FE], frame: &Frame<Stark252PrimeField>) {
    let curr = frame.get_row(0);
    let one = FE::one();

    constraints[MUL_1] = &curr[FRAME_MUL] - (&curr[FRAME_OP0] * &curr[FRAME_OP1]);

    constraints[MUL_2] = &curr[F_RES_ADD] * (&curr[FRAME_OP0] + &curr[FRAME_OP1])
        + &curr[F_RES_MUL] * &curr[FRAME_MUL]
        + (&one - &curr[F_RES_ADD] - &curr[F_RES_MUL] - &curr[F_PC_JNZ]) * &curr[FRAME_OP1]
        - (&one - &curr[F_PC_JNZ]) * &curr[FRAME_RES];

    constraints[CALL_1] = &curr[F_OPC_CALL] * (&curr[FRAME_DST] - &curr[FRAME_FP]);

    constraints[CALL_2] =
        &curr[F_OPC_CALL] * (&curr[FRAME_OP0] - (&curr[FRAME_PC] + frame_inst_size(curr)));

    constraints[ASSERT_EQ] = &curr[F_OPC_AEQ] * (&curr[FRAME_DST] - &curr[FRAME_RES]);
}

fn enforce_selector(constraints: &mut [FE], frame: &Frame<Stark252PrimeField>) {
    let curr = frame.get_row(0);
    for result_cell in constraints.iter_mut().take(ASSERT_EQ + 1).skip(INST) {
        *result_cell = result_cell.clone() * curr[FRAME_SELECTOR].clone();
    }
}

fn memory_is_increasing(
    constraints: &mut [FE],
    frame: &Frame<Stark252PrimeField>,
    builtin_offset: usize,
) {
    let curr = frame.get_row(0);
    let next = frame.get_row(1);
    let one = FieldElement::one();

    constraints[MEMORY_INCREASING_0] = (&curr[MEMORY_ADDR_SORTED_0 - builtin_offset]
        - &curr[MEMORY_ADDR_SORTED_1 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_1 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_0 - builtin_offset]
            - &one);

    constraints[MEMORY_INCREASING_1] = (&curr[MEMORY_ADDR_SORTED_1 - builtin_offset]
        - &curr[MEMORY_ADDR_SORTED_2 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_2 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_1 - builtin_offset]
            - &one);

    constraints[MEMORY_INCREASING_2] = (&curr[MEMORY_ADDR_SORTED_2 - builtin_offset]
        - &curr[MEMORY_ADDR_SORTED_3 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_3 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_2 - builtin_offset]
            - &one);

    constraints[MEMORY_INCREASING_3] = (&curr[MEMORY_ADDR_SORTED_3 - builtin_offset]
        - &next[MEMORY_ADDR_SORTED_0 - builtin_offset])
        * (&next[MEMORY_ADDR_SORTED_0 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_3 - builtin_offset]
            - &one);

    constraints[MEMORY_CONSISTENCY_0] = (&curr[MEMORY_VALUES_SORTED_0 - builtin_offset]
        - &curr[MEMORY_VALUES_SORTED_1 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_1 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_0 - builtin_offset]
            - &one);

    constraints[MEMORY_CONSISTENCY_1] = (&curr[MEMORY_VALUES_SORTED_1 - builtin_offset]
        - &curr[MEMORY_VALUES_SORTED_2 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_2 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_1 - builtin_offset]
            - &one);

    constraints[MEMORY_CONSISTENCY_2] = (&curr[MEMORY_VALUES_SORTED_2 - builtin_offset]
        - &curr[MEMORY_VALUES_SORTED_3 - builtin_offset])
        * (&curr[MEMORY_ADDR_SORTED_3 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_2 - builtin_offset]
            - &one);

    constraints[MEMORY_CONSISTENCY_3] = (&curr[MEMORY_VALUES_SORTED_3 - builtin_offset]
        - &next[MEMORY_VALUES_SORTED_0 - builtin_offset])
        * (&next[MEMORY_ADDR_SORTED_0 - builtin_offset]
            - &curr[MEMORY_ADDR_SORTED_3 - builtin_offset]
            - &one);
}

fn permutation_argument(
    constraints: &mut [FE],
    frame: &Frame<Stark252PrimeField>,
    rap_challenges: &CairoRAPChallenges,
    builtin_offset: usize,
) {
    let curr = frame.get_row(0);
    let next = frame.get_row(1);
    let z = &rap_challenges.z_memory;
    let alpha = &rap_challenges.alpha_memory;

    let p0 = &curr[PERMUTATION_ARGUMENT_COL_0 - builtin_offset];
    let p0_next = &next[PERMUTATION_ARGUMENT_COL_0 - builtin_offset];
    let p1 = &curr[PERMUTATION_ARGUMENT_COL_1 - builtin_offset];
    let p2 = &curr[PERMUTATION_ARGUMENT_COL_2 - builtin_offset];
    let p3 = &curr[PERMUTATION_ARGUMENT_COL_3 - builtin_offset];

    let ap0_next = &next[MEMORY_ADDR_SORTED_0 - builtin_offset];
    let ap1 = &curr[MEMORY_ADDR_SORTED_1 - builtin_offset];
    let ap2 = &curr[MEMORY_ADDR_SORTED_2 - builtin_offset];
    let ap3 = &curr[MEMORY_ADDR_SORTED_3 - builtin_offset];

    let vp0_next = &next[MEMORY_VALUES_SORTED_0 - builtin_offset];
    let vp1 = &curr[MEMORY_VALUES_SORTED_1 - builtin_offset];
    let vp2 = &curr[MEMORY_VALUES_SORTED_2 - builtin_offset];
    let vp3 = &curr[MEMORY_VALUES_SORTED_3 - builtin_offset];

    let a0_next = &next[FRAME_PC];
    let a1 = &curr[FRAME_DST_ADDR];
    let a2 = &curr[FRAME_OP0_ADDR];
    let a3 = &curr[FRAME_OP1_ADDR];

    let v0_next = &next[FRAME_INST];
    let v1 = &curr[FRAME_DST];
    let v2 = &curr[FRAME_OP0];
    let v3 = &curr[FRAME_OP1];

    constraints[PERMUTATION_ARGUMENT_0] =
        (z - (ap1 + alpha * vp1)) * p1 - (z - (a1 + alpha * v1)) * p0;
    constraints[PERMUTATION_ARGUMENT_1] =
        (z - (ap2 + alpha * vp2)) * p2 - (z - (a2 + alpha * v2)) * p1;
    constraints[PERMUTATION_ARGUMENT_2] =
        (z - (ap3 + alpha * vp3)) * p3 - (z - (a3 + alpha * v3)) * p2;
    constraints[PERMUTATION_ARGUMENT_3] =
        (z - (ap0_next + alpha * vp0_next)) * p0_next - (z - (a0_next + alpha * v0_next)) * p3;
}

fn permutation_argument_range_check(
    constraints: &mut [FE],
    frame: &Frame<Stark252PrimeField>,
    rap_challenges: &CairoRAPChallenges,
    builtin_offset: usize,
) {
    let curr = frame.get_row(0);
    let next = frame.get_row(1);
    let one = FieldElement::one();
    let z = &rap_challenges.z_range_check;

    constraints[RANGE_CHECK_INCREASING_0] = (&curr[RANGE_CHECK_COL_1 - builtin_offset]
        - &curr[RANGE_CHECK_COL_2 - builtin_offset])
        * (&curr[RANGE_CHECK_COL_2 - builtin_offset]
            - &curr[RANGE_CHECK_COL_1 - builtin_offset]
            - &one);
    constraints[RANGE_CHECK_INCREASING_1] = (&curr[RANGE_CHECK_COL_2 - builtin_offset]
        - &curr[RANGE_CHECK_COL_3 - builtin_offset])
        * (&curr[RANGE_CHECK_COL_3 - builtin_offset]
            - &curr[RANGE_CHECK_COL_2 - builtin_offset]
            - &one);
    constraints[RANGE_CHECK_INCREASING_2] = (&curr[RANGE_CHECK_COL_3 - builtin_offset]
        - &next[RANGE_CHECK_COL_1 - builtin_offset])
        * (&next[RANGE_CHECK_COL_1 - builtin_offset]
            - &curr[RANGE_CHECK_COL_3 - builtin_offset]
            - &one);

    let p0 = &curr[PERMUTATION_ARGUMENT_RANGE_CHECK_COL_1 - builtin_offset];
    let p0_next = &next[PERMUTATION_ARGUMENT_RANGE_CHECK_COL_1 - builtin_offset];
    let p1 = &curr[PERMUTATION_ARGUMENT_RANGE_CHECK_COL_2 - builtin_offset];
    let p2 = &curr[PERMUTATION_ARGUMENT_RANGE_CHECK_COL_3 - builtin_offset];

    let ap0_next = &next[RANGE_CHECK_COL_1 - builtin_offset];
    let ap1 = &curr[RANGE_CHECK_COL_2 - builtin_offset];
    let ap2 = &curr[RANGE_CHECK_COL_3 - builtin_offset];

    let a0_next = &next[OFF_DST];
    let a1 = &curr[OFF_OP0];
    let a2 = &curr[OFF_OP1];

    constraints[RANGE_CHECK_0] = (z - ap1) * p1 - (z - a1) * p0;
    constraints[RANGE_CHECK_1] = (z - ap2) * p2 - (z - a2) * p1;
    constraints[RANGE_CHECK_2] = (z - ap0_next) * p0_next - (z - a0_next) * p2;
}

fn frame_inst_size(frame_row: &[FE]) -> FE {
    &frame_row[F_OP_1_VAL] + FE::one()
}

fn range_check_builtin(
    constraints: &mut [FieldElement<Stark252PrimeField>],
    frame: &Frame<Stark252PrimeField>,
) {
    let curr = frame.get_row(0);

    constraints[RANGE_CHECK_BUILTIN] = evaluate_range_check_builtin_constraint(curr)
}

fn evaluate_range_check_builtin_constraint(curr: &[FE]) -> FE {
    &curr[RC_0]
        + &curr[RC_1] * &FE::from_hex("10000").unwrap()
        + &curr[RC_2] * &FE::from_hex("100000000").unwrap()
        + &curr[RC_3] * &FE::from_hex("1000000000000").unwrap()
        + &curr[RC_4] * &FE::from_hex("10000000000000000").unwrap()
        + &curr[RC_5] * &FE::from_hex("100000000000000000000").unwrap()
        + &curr[RC_6] * &FE::from_hex("1000000000000000000000000").unwrap()
        + &curr[RC_7] * &FE::from_hex("10000000000000000000000000000").unwrap()
        - &curr[RC_VALUE]
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod test {
    use crate::{
        cairo::runner::run::{cairo0_program_path, generate_prover_args, CairoVersion},
        starks::{debug::validate_trace, domain::Domain},
    };

    use super::*;
    use lambdaworks_crypto::fiat_shamir::default_transcript::DefaultTranscript;
    use lambdaworks_math::field::element::FieldElement;

    #[test]
    fn range_check_eval_works() {
        let mut row: Vec<FE> = Vec::new();

        for _ in 0..61 {
            row.push(FE::zero());
        }

        row[super::RC_0] = FE::one();
        row[super::RC_1] = FE::one();
        row[super::RC_2] = FE::one();
        row[super::RC_3] = FE::one();
        row[super::RC_4] = FE::one();
        row[super::RC_5] = FE::one();
        row[super::RC_6] = FE::one();
        row[super::RC_7] = FE::one();

        row[super::RC_VALUE] = FE::from_hex("00010001000100010001000100010001").unwrap();
        assert_eq!(evaluate_range_check_builtin_constraint(&row), FE::zero());
    }

    #[test]
    fn check_simple_cairo_trace_evaluates_to_zero() {
        let program_content = std::fs::read(cairo0_program_path("simple_program.json")).unwrap();
        let (main_trace, cairo_air, public_input) =
            generate_prover_args(&program_content, &CairoVersion::V0, &None, 1).unwrap();
        let mut trace_polys = main_trace.compute_trace_polys();
        let mut transcript = DefaultTranscript::new();
        let rap_challenges = cairo_air.build_rap_challenges(&mut transcript);

        let aux_trace =
            cairo_air.build_auxiliary_trace(&main_trace, &rap_challenges, &public_input);
        let aux_polys = aux_trace.compute_trace_polys();

        trace_polys.extend_from_slice(&aux_polys);

        let domain = Domain::new(&cairo_air);

        assert!(validate_trace(
            &cairo_air,
            &trace_polys,
            &domain,
            &public_input,
            &rap_challenges
        ));
    }

    #[test]
    fn test_build_auxiliary_trace_add_program_in_public_input_section_works() {
        let dummy_public_input = PublicInputs {
            pc_init: FieldElement::zero(),
            ap_init: FieldElement::zero(),
            fp_init: FieldElement::zero(),
            pc_final: FieldElement::zero(),
            ap_final: FieldElement::zero(),
            public_memory: HashMap::from([
                (FieldElement::one(), FieldElement::from(10)),
                (FieldElement::from(2), FieldElement::from(20)),
                (FieldElement::from(3), FieldElement::from(30)),
            ]),
            range_check_max: None,
            range_check_min: None,
            num_steps: 1,
            memory_segments: MemorySegmentMap::new(),
        };

        let a = vec![
            FieldElement::one(),
            FieldElement::one(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
        ];
        let v = vec![
            FieldElement::one(),
            FieldElement::one(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
        ];
        let (ap, vp) = add_pub_memory_in_public_input_section(&a, &v, &dummy_public_input);
        assert_eq!(
            ap,
            vec![
                FieldElement::one(),
                FieldElement::one(),
                FieldElement::zero(),
                FieldElement::one(),
                FieldElement::from(2),
                FieldElement::from(3)
            ]
        );
        assert_eq!(
            vp,
            vec![
                FieldElement::one(),
                FieldElement::one(),
                FieldElement::zero(),
                FieldElement::from(10),
                FieldElement::from(20),
                FieldElement::from(30)
            ]
        );
    }

    #[test]
    fn test_build_auxiliary_trace_add_program_with_output_in_public_input_section_works() {
        let dummy_public_input = PublicInputs {
            pc_init: FieldElement::zero(),
            ap_init: FieldElement::zero(),
            fp_init: FieldElement::zero(),
            pc_final: FieldElement::zero(),
            ap_final: FieldElement::zero(),
            public_memory: HashMap::from([
                (FieldElement::one(), FieldElement::from(10)),
                (FieldElement::from(2), FieldElement::from(20)),
                (FieldElement::from(3), FieldElement::from(30)),
                (FieldElement::from(20), FieldElement::from(40)),
                (FieldElement::from(21), FieldElement::from(50)),
            ]),
            range_check_max: None,
            range_check_min: None,
            num_steps: 1,
            memory_segments: MemorySegmentMap::from([(MemorySegment::Output, 20..22)]),
        };

        let a = vec![
            FieldElement::one(),
            FieldElement::one(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
        ];
        let v = vec![
            FieldElement::one(),
            FieldElement::one(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
        ];
        let (ap, vp) = add_pub_memory_in_public_input_section(&a, &v, &dummy_public_input);
        assert_eq!(
            ap,
            vec![
                FieldElement::one(),
                FieldElement::one(),
                FieldElement::zero(),
                FieldElement::one(),
                FieldElement::from(2),
                FieldElement::from(3),
                FieldElement::from(20),
                FieldElement::from(21)
            ]
        );
        assert_eq!(
            vp,
            vec![
                FieldElement::one(),
                FieldElement::one(),
                FieldElement::zero(),
                FieldElement::from(10),
                FieldElement::from(20),
                FieldElement::from(30),
                FieldElement::from(40),
                FieldElement::from(50)
            ]
        );
    }

    #[test]
    fn test_build_auxiliary_trace_sort_columns_by_memory_address() {
        let a = vec![
            FieldElement::from(2),
            FieldElement::one(),
            FieldElement::from(3),
            FieldElement::from(2),
        ];
        let v = vec![
            FieldElement::from(6),
            FieldElement::from(4),
            FieldElement::from(5),
            FieldElement::from(6),
        ];
        let (ap, vp) = sort_columns_by_memory_address(a, v);
        assert_eq!(
            ap,
            vec![
                FieldElement::one(),
                FieldElement::from(2),
                FieldElement::from(2),
                FieldElement::from(3)
            ]
        );
        assert_eq!(
            vp,
            vec![
                FieldElement::from(4),
                FieldElement::from(6),
                FieldElement::from(6),
                FieldElement::from(5),
            ]
        );
    }

    #[test]
    fn test_build_auxiliary_trace_generate_permutation_argument_column() {
        let a = vec![
            FieldElement::from(3),
            FieldElement::one(),
            FieldElement::from(2),
        ];
        let v = vec![
            FieldElement::from(5),
            FieldElement::one(),
            FieldElement::from(2),
        ];
        let ap = vec![
            FieldElement::one(),
            FieldElement::from(2),
            FieldElement::from(3),
        ];
        let vp = vec![
            FieldElement::one(),
            FieldElement::from(2),
            FieldElement::from(5),
        ];
        let rap_challenges = CairoRAPChallenges {
            alpha_memory: FieldElement::from(15),
            z_memory: FieldElement::from(10),
            z_range_check: FieldElement::zero(),
        };
        let p = generate_memory_permutation_argument_column(a, v, &ap, &vp, &rap_challenges);
        assert_eq!(
            p,
            vec![
                FieldElement::from_hex(
                    "2aaaaaaaaaaaab0555555555555555555555555555555555555555555555561"
                )
                .unwrap(),
                FieldElement::from_hex(
                    "1745d1745d174602e8ba2e8ba2e8ba2e8ba2e8ba2e8ba2e8ba2e8ba2e8ba2ec"
                )
                .unwrap(),
                FieldElement::one(),
            ]
        );
    }
}
