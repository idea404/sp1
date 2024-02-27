pub mod event;

use core::borrow::Borrow;
use core::borrow::BorrowMut;
use core::mem::size_of;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, Field, PrimeField};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use sp1_derive::AlignedBorrow;
use p3_maybe_rayon::prelude::*; //{ParallelIterator, ParallelSlice,};
use crate::air::FieldAirBuilder;
use crate::air::MachineAir;
use crate::air::SP1AirBuilder;
use crate::runtime::ExecutionRecord;
use crate::utils::pad_to_power_of_two;

use tracing::instrument;

/// The number of main trace columns for `FieldLTUChip`.
pub const NUM_FIELD_COLS: usize = size_of::<FieldLTUCols<u8>>();
const WIDTH:usize = 4;
/// A chip that implements less than within the field.
#[derive(Default)]
pub struct FieldLTUChip;

/// The column layout for the chip.
#[derive(Debug, Clone, Copy, AlignedBorrow)]
#[repr(C)]
pub struct FieldLTUCols<T> {
    /// The result of the `LT` operation on `a` and `b`
    pub lt: T,

    /// The first field operand.
    pub b: T,

    /// The second field operand.
    pub c: T,

    /// The difference between `b` and `c` in little-endian order.
    pub diff_bits: [T; LTU_NB_BITS + 1],

    // TODO:  Support multiplicities > 1.  Right now there can be duplicate rows.
    // pub multiplicities: T,
    pub is_real: T,
}
unsafe impl<T> Send for FieldLTUCols<T> {}
unsafe impl<T> Sync for FieldLTUCols<T> {}
#[derive(Debug, Clone, AlignedBorrow, Copy)]
#[repr(C)]
pub struct PackedFieldLTUCols<T>{
    packed_chips: [FieldLTUCols<T>;WIDTH]
}

impl<F: PrimeField> MachineAir<F> for FieldLTUChip {
    fn name(&self) -> String {
        "FieldLTU".to_string()
    }

    #[instrument(name = "generate FieldLTU trace", skip_all)]
    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        _output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        // Generate the trace rows for each event.
        let rows = input
            .field_events
            .par_chunks_exact(WIDTH)
            .map(|events| {
                let mut row = [F::zero(); NUM_FIELD_COLS * WIDTH];
                let packed_cols: &mut PackedFieldLTUCols<F> = row.as_mut_slice().borrow_mut();
		for (i,event) in events.iter().enumerate(){
		    let mut cols = packed_cols.packed_chips[i];
                    let diff = event.b.wrapping_sub(event.c).wrapping_add(1 << LTU_NB_BITS);
                    cols.b = F::from_canonical_u32(event.b);
                    cols.c = F::from_canonical_u32(event.c);
                    for i in 0..cols.diff_bits.len() {
			cols.diff_bits[i] = F::from_canonical_u32((diff >> i) & 1);
                    }
                    let max = 1 << LTU_NB_BITS;
                    if diff >= max {
			panic!("diff overflow");
                    }
                    cols.lt = F::from_bool(event.ltu);
                    cols.is_real = F::one();
		}
		row
            })
            .collect::<Vec<_>>();

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_FIELD_COLS*WIDTH,
        );

        // Pad the trace to a power of two.
	const width : usize = NUM_FIELD_COLS*WIDTH;
        pad_to_power_of_two::<width, F>(&mut trace.values);

        trace
    }
}

pub const LTU_NB_BITS: usize = 29;

impl<F: Field> BaseAir<F> for FieldLTUChip {
    fn width(&self) -> usize {
        NUM_FIELD_COLS*WIDTH
    }
}

impl<AB: SP1AirBuilder> Air<AB> for FieldLTUChip {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local_packed: &PackedFieldLTUCols<AB::Var> = main.row_slice(0).borrow();
	let local_packed_chips: Vec<FieldLTUCols<AB::Var>> = local_packed.packed_chips.to_vec();
	local_packed_chips.iter().for_each(|local| {
            // Dummy constraint for normalizing to degree 3.
            builder.assert_eq(local.b * local.b * local.b, local.b * local.b * local.b);

            // Verify that lt is a boolean.
            builder.assert_bool(local.lt);

            // Verify that the diff bits are boolean.
            for i in 0..local.diff_bits.len() {
		builder.assert_bool(local.diff_bits[i]);
            }

            // Verify the decomposition of b - c.
            let mut diff = AB::Expr::zero();
            for i in 0..local.diff_bits.len() {
		diff += local.diff_bits[i] * AB::F::from_canonical_u32(1 << i);
            }
            builder.when(local.is_real).assert_eq(
		local.b - local.c + AB::F::from_canonical_u32(1 << LTU_NB_BITS),
		diff,
            );

            // Assert that the output is correct.
            builder
		.when(local.is_real)
		.assert_eq(local.lt, AB::Expr::one() - local.diff_bits[LTU_NB_BITS]);

            // Receive the field operation.
            builder.receive_field_op(local.lt, local.b, local.c, local.is_real);
	});
    }
}
