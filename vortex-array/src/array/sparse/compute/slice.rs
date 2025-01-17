use vortex_error::VortexResult;

use crate::array::sparse::SparseArray;
use crate::compute::{slice, SliceFn};
use crate::{Array, IntoArray};

impl SliceFn for SparseArray {
    fn slice(&self, start: usize, stop: usize) -> VortexResult<Array> {
        // Find the index of the first patch index that is greater than or equal to the offset of this array
        let index_start_index = self.search_index(start)?.to_index();
        let index_end_index = self.search_index(stop)?.to_index();

        Ok(Self::try_new_with_offset(
            slice(self.indices(), index_start_index, index_end_index)?,
            slice(self.values(), index_start_index, index_end_index)?,
            stop - start,
            self.indices_offset() + start,
            self.fill_value().clone(),
        )?
        .into_array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IntoArrayVariant;

    #[test]
    fn test_slice() {
        let values = vec![15_u32, 135, 13531, 42].into_array();
        let indices = vec![10_u64, 11, 50, 100].into_array();

        let sparse = SparseArray::try_new(indices.clone(), values, 101, 0_u32.into())
            .unwrap()
            .into_array();

        let sliced = slice(&sparse, 15, 100).unwrap();
        assert_eq!(sliced.len(), 100 - 15);
        let primitive = SparseArray::try_from(sliced)
            .unwrap()
            .values()
            .into_primitive()
            .unwrap();

        assert_eq!(primitive.maybe_null_slice::<u32>(), &[13531]);
    }

    #[test]
    fn doubly_sliced() {
        let values = vec![15_u32, 135, 13531, 42].into_array();
        let indices = vec![10_u64, 11, 50, 100].into_array();

        let sparse = SparseArray::try_new(indices.clone(), values, 101, 0_u32.into())
            .unwrap()
            .into_array();

        let sliced = slice(&sparse, 15, 100).unwrap();
        assert_eq!(sliced.len(), 100 - 15);
        let primitive = SparseArray::try_from(sliced.clone())
            .unwrap()
            .values()
            .into_primitive()
            .unwrap();

        assert_eq!(primitive.maybe_null_slice::<u32>(), &[13531]);

        let doubly_sliced = slice(&sliced, 35, 36).unwrap();
        let primitive_doubly_sliced = SparseArray::try_from(doubly_sliced)
            .unwrap()
            .values()
            .into_primitive()
            .unwrap();

        assert_eq!(primitive_doubly_sliced.maybe_null_slice::<u32>(), &[13531]);
    }
}
