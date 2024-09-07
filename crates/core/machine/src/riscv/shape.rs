use core::panic;
use std::iter::once;

use itertools::Itertools;

use hashbrown::HashMap;
use p3_field::PrimeField32;
use sp1_core_executor::{CoreShape, ExecutionRecord, Program};
use sp1_stark::{air::MachineAir, ProofShape};

use crate::memory::{MemoryChipType, MemoryProgramChip};

use super::{
    AddSubChip, BitwiseChip, CpuChip, DivRemChip, LtChip, MemoryChip, MulChip, ProgramChip,
    RiscvAir, ShiftLeft, ShiftRightChip,
};

/// A structure that enables fixing the shape of an executionrecord.
pub struct CoreShapeConfig<F: PrimeField32> {
    allowed_log_heights: HashMap<RiscvAir<F>, Vec<usize>>,
}

impl<F: PrimeField32> CoreShapeConfig<F> {
    /// Fix the preprocessed shape of the proof.
    pub fn fix_preprocessed_shape(&self, program: &mut Program) {
        if program.preprocessed_shape.is_some() {
            tracing::warn!("preprocessed shape already fixed");
            // TODO: Change this to not panic (i.e. return);
            panic!("cannot fix preprocessed shape twice");
        }

        let shape = RiscvAir::<F>::preprocessed_heights(program)
            .into_iter()
            .map(|(air, height)| {
                for &allowed_log_height in self.allowed_log_heights.get(&air).unwrap() {
                    let allowed_height = 1 << allowed_log_height;
                    if height <= allowed_height {
                        return (air.name(), allowed_log_height);
                    }
                }
                panic!("air {} not allowed at height {}", air.name(), height);
            })
            .collect();

        let shape = CoreShape { inner: shape };
        program.preprocessed_shape = Some(shape);
    }

    /// Fix the shape of the proof.
    pub fn fix_shape(&self, record: &mut ExecutionRecord) {
        if record.program.preprocessed_shape.is_none() {
            panic!("program shape not set");
        }
        if record.shape.is_some() {
            tracing::warn!("shape already fixed");
            // TODO: Change this to not panic (i.e. return);
            panic!("cannot fix shape twice");
        }

        let shape = RiscvAir::<F>::heights(record)
            .into_iter()
            .map(|(air, height)| {
                for &allowed_log_height in self.allowed_log_heights.get(&air).unwrap() {
                    let allowed_height = 1 << allowed_log_height;
                    if height <= allowed_height {
                        return (air.name(), allowed_log_height);
                    }
                }
                panic!("air {} not allowed at height {}", air.name(), height);
            })
            .collect();

        let shape = CoreShape { inner: shape };
        record.shape = Some(shape);
    }

    pub fn generate_all_allowed_shapes(&self) -> impl Iterator<Item = ProofShape> + '_ {
        // for chip in allowed_heights.
        self.allowed_log_heights
            .iter()
            .map(|(chip, heights)| {
                let name = chip.name();
                once((name.clone(), None))
                    .chain(heights.iter().map(move |height| (name.clone(), Some(*height))))
            })
            .multi_cartesian_product()
            .map(|iter| {
                iter.into_iter()
                    .filter_map(|(name, maybe_height)| {
                        maybe_height.map(|log_height| (name, log_height))
                    })
                    .collect::<ProofShape>()
            })
    }
}

impl<F: PrimeField32> Default for CoreShapeConfig<F> {
    fn default() -> Self {
        let mut allowed_heights = HashMap::new();

        // Preprocessed chip heights.
        let program_heights = vec![10, 16, 21, 22];
        let program_memory_heights = vec![10, 16, 21, 22];

        // Core chip heights.
        let cpu_heights = vec![16, 20, 21, 22];
        let divrem_heights = vec![10, 16, 20, 21, 22];
        let add_sub_heights = vec![10, 16, 20, 21, 22];
        let bitwise_heights = vec![10, 16, 20, 21, 22];
        let mul_heights = vec![10, 16, 20, 21, 22];
        let shift_right_heights = vec![10, 18, 20, 21, 22];
        let shift_left_heights = vec![10, 18, 20, 21, 22];
        let lt_heights = vec![10, 18, 20, 21, 22];
        let memory_init_heights = vec![10, 18, 20, 21, 22];
        let memory_final_heights = vec![10, 18, 20, 21, 22];

        // Get allowed heights for preprocessed chips.
        allowed_heights.extend([
            (RiscvAir::Program(ProgramChip::default()), program_heights),
            (RiscvAir::ProgramMemory(MemoryProgramChip::default()), program_memory_heights),
        ]);

        // Get the heights of core chips.

        allowed_heights.extend([
            (RiscvAir::Cpu(CpuChip::default()), cpu_heights),
            (RiscvAir::DivRem(DivRemChip::default()), divrem_heights),
            (RiscvAir::Add(AddSubChip::default()), add_sub_heights),
            (RiscvAir::Bitwise(BitwiseChip::default()), bitwise_heights),
            (RiscvAir::Mul(MulChip::default()), mul_heights),
            (RiscvAir::ShiftRight(ShiftRightChip::default()), shift_right_heights),
            (RiscvAir::ShiftLeft(ShiftLeft::default()), shift_left_heights),
            (RiscvAir::Lt(LtChip::default()), lt_heights),
            (
                RiscvAir::MemoryInit(MemoryChip::new(MemoryChipType::Initialize)),
                memory_init_heights,
            ),
            (
                RiscvAir::MemoryFinal(MemoryChip::new(MemoryChipType::Finalize)),
                memory_final_heights,
            ),
        ]);

        Self { allowed_log_heights: allowed_heights }
    }
}

#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;

    use super::*;

    #[test]
    #[ignore]
    fn test_making_shapes() {
        let shape_config = CoreShapeConfig::<BabyBear>::default();
        let num_shapes = shape_config.generate_all_allowed_shapes().take(1 << 25).count();

        println!("There are {} core shapes", num_shapes);
    }
}