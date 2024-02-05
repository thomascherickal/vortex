use std::any::Any;
use std::sync::{Arc, RwLock};

use croaring::{Bitmap, Native};

use compress::roaring_encode;
use enc::array::{
    check_index_bounds, check_slice_bounds, Array, ArrayKind, ArrayRef, ArrowIterator, Encoding,
    EncodingId, EncodingRef,
};
use enc::compress::EncodingCompression;
use enc::dtype::DType;
use enc::dtype::Nullability::NonNullable;
use enc::error::{EncError, EncResult};
use enc::formatter::{ArrayDisplay, ArrayFormatter};
use enc::scalar::Scalar;
use enc::stats::{Stats, StatsSet};

mod compress;
mod stats;

#[derive(Debug, Clone)]
pub struct RoaringBoolArray {
    bitmap: Bitmap,
    length: usize,
    stats: Arc<RwLock<StatsSet>>,
}

impl RoaringBoolArray {
    pub fn new(bitmap: Bitmap, length: usize) -> Self {
        Self {
            bitmap,
            length,
            stats: Arc::new(RwLock::new(StatsSet::new())),
        }
    }

    pub fn bitmap(&self) -> &Bitmap {
        &self.bitmap
    }

    pub fn encode(array: &dyn Array) -> EncResult<Self> {
        match ArrayKind::from(array) {
            ArrayKind::Bool(p) => Ok(roaring_encode(p)),
            _ => Err(EncError::InvalidEncoding(array.encoding().id().clone())),
        }
    }
}

impl Array for RoaringBoolArray {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn boxed(self) -> ArrayRef {
        Box::new(self)
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn len(&self) -> usize {
        self.length
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    fn dtype(&self) -> &DType {
        &DType::Bool(NonNullable)
    }

    fn stats(&self) -> Stats {
        Stats::new(&self.stats, self)
    }

    fn scalar_at(&self, index: usize) -> EncResult<Box<dyn Scalar>> {
        check_index_bounds(self, index)?;

        if self.bitmap.contains(index as u32) {
            Ok(true.into())
        } else {
            Ok(false.into())
        }
    }

    fn iter_arrow(&self) -> Box<ArrowIterator> {
        todo!()
    }

    fn slice(&self, start: usize, stop: usize) -> EncResult<ArrayRef> {
        check_slice_bounds(self, start, stop)?;

        let slice_bitmap = Bitmap::from_range(start as u32..stop as u32);
        let bitmap = self.bitmap.and(&slice_bitmap).add_offset(-(start as i64));

        Ok(Self {
            bitmap,
            length: stop - start,
            stats: Arc::new(RwLock::new(StatsSet::new())),
        }
        .boxed())
    }

    #[inline]
    fn encoding(&self) -> EncodingRef {
        &RoaringBoolEncoding
    }

    #[inline]
    fn nbytes(&self) -> usize {
        // TODO(ngates): do we want Native serializer? Or portable? Or frozen?
        self.bitmap.get_serialized_size_in_bytes::<Native>()
    }
}

impl<'arr> AsRef<(dyn Array + 'arr)> for RoaringBoolArray {
    fn as_ref(&self) -> &(dyn Array + 'arr) {
        self
    }
}

impl ArrayDisplay for RoaringBoolArray {
    fn fmt(&self, f: &mut ArrayFormatter) -> std::fmt::Result {
        f.writeln("roaring:")?;
        f.indent(|indent| indent.writeln(format!("{:?}", self.bitmap)))
    }
}

#[derive(Debug)]
pub struct RoaringBoolEncoding;

pub const ROARING_BOOL_ENCODING: EncodingId = EncodingId::new("roaring.bool");

impl Encoding for RoaringBoolEncoding {
    fn id(&self) -> &EncodingId {
        &ROARING_BOOL_ENCODING
    }

    fn compression(&self) -> Option<&dyn EncodingCompression> {
        Some(self)
    }
}

#[cfg(test)]
mod test {
    use enc::array::bool::BoolArray;
    use enc::array::Array;
    use enc::error::EncResult;
    use enc::scalar::Scalar;

    use crate::RoaringBoolArray;

    #[test]
    pub fn iter() -> EncResult<()> {
        let bool: &dyn Array = &BoolArray::from(vec![true, false, true, true]);
        let array = RoaringBoolArray::encode(bool)?;

        let values = array.bitmap().to_vec();
        assert_eq!(values, vec![0, 2, 3]);

        Ok(())
    }

    #[test]
    pub fn scalar_at() -> EncResult<()> {
        let bool: &dyn Array = &BoolArray::from(vec![true, false, true, true]);
        let array = RoaringBoolArray::encode(bool)?;

        let truthy: Box<dyn Scalar> = true.into();
        let falsy: Box<dyn Scalar> = false.into();

        assert_eq!(array.scalar_at(0)?, truthy);
        assert_eq!(array.scalar_at(1)?, falsy);
        assert_eq!(array.scalar_at(2)?, truthy);
        assert_eq!(array.scalar_at(3)?, truthy);

        Ok(())
    }
}