use log::debug;

use codecz::alp;
use codecz::alp::{ALPEncoded, ALPExponents, SupportsALP};
use enc::array::primitive::PrimitiveArray;
use enc::array::sparse::SparseArray;
use enc::array::{Array, ArrayRef};
use enc::compress::{CompressConfig, CompressCtx, Compressor, EncodingCompression};
use enc::ptype::{NativePType, PType};

use crate::alp::{ALPArray, ALPEncoding};

impl EncodingCompression for ALPEncoding {
    fn compressor(
        &self,
        array: &dyn Array,
        _config: &CompressConfig,
    ) -> Option<&'static Compressor> {
        // Only support primitive arrays
        let Some(parray) = array.as_any().downcast_ref::<PrimitiveArray>() else {
            debug!("Skipping ALP: not primitive");
            return None;
        };

        // Only supports f32 and f64
        if !matches!(parray.ptype(), PType::F32 | PType::F64) {
            debug!("Skipping ALP: only supports f32 and f64");
            return None;
        }

        Some(&(alp_compressor as Compressor))
    }
}

fn alp_compressor(array: &dyn Array, like: Option<&dyn Array>, ctx: CompressCtx) -> ArrayRef {
    let like_alp = like.and_then(|like_array| like_array.as_any().downcast_ref::<ALPArray>());

    let parray = array.as_any().downcast_ref::<PrimitiveArray>().unwrap();
    let (encoded, exponents, patches) = like_alp
        .map(|alp_like| alp_encode_like_parts(parray, alp_like))
        .unwrap_or_else(|| alp_encode_parts(parray));

    ALPArray::new(
        ctx.next_level()
            .compress(encoded.as_ref(), like_alp.map(|a| a.encoded())),
        exponents,
        patches.map(|p| {
            ctx.next_level().compress(
                p.as_ref(),
                like_alp.and_then(|a| a.patches()).map(|p| p.as_ref()),
            )
        }),
    )
    .boxed()
}

pub fn alp_encode(parray: &PrimitiveArray) -> ArrayRef {
    let (encoded, exponents, patches) = alp_encode_parts(parray);
    ALPArray::new(encoded, exponents, patches).boxed()
}

fn alp_encode_parts(parray: &PrimitiveArray) -> (ArrayRef, ALPExponents, Option<ArrayRef>) {
    match parray.ptype() {
        PType::F32 => {
            alp_encode_primitive(parray.buffer().typed_data::<f32>(), parray.validity(), None)
        }
        PType::F64 => {
            alp_encode_primitive(parray.buffer().typed_data::<f64>(), parray.validity(), None)
        }
        _ => panic!("Unsupported ptype"),
    }
}

fn alp_encode_like_parts(
    parray: &PrimitiveArray,
    sample: &ALPArray,
) -> (ArrayRef, ALPExponents, Option<ArrayRef>) {
    match parray.ptype() {
        PType::F32 => alp_encode_primitive(
            parray.buffer().typed_data::<f32>(),
            parray.validity(),
            Some(sample.exponents()),
        ),
        PType::F64 => alp_encode_primitive(
            parray.buffer().typed_data::<f64>(),
            parray.validity(),
            Some(sample.exponents()),
        ),
        _ => panic!("Unsupported ptype"),
    }
}

fn alp_encode_primitive<T: SupportsALP + NativePType>(
    values: &[T],
    validity: Option<&ArrayRef>,
    exponents: Option<ALPExponents>,
) -> (ArrayRef, ALPExponents, Option<ArrayRef>)
where
    T::EncInt: NativePType,
{
    // TODO: actually handle CodecErrors instead of blindly unwrapping
    let ALPEncoded {
        values,
        exponents,
        exceptions_idx,
        num_exceptions,
    } = exponents
        .map(|exp| alp::encode_with(values, exp))
        .unwrap_or_else(|| alp::encode(values))
        .unwrap();
    let values = PrimitiveArray::from_nullable_in(values, validity.cloned()); // move and re-alias

    let patches = if num_exceptions == 0 {
        None
    } else {
        let patch_indices = codecz::utils::into_u32_vec(&exceptions_idx, num_exceptions);
        let patch_values = codecz::utils::gather_patches(
            values.buffer().typed_data::<T>(),
            patch_indices.as_slice(),
        );
        Some(
            SparseArray::new(
                PrimitiveArray::from_vec_in(patch_indices).boxed(),
                PrimitiveArray::from_vec_in(patch_values).boxed(),
                values.len(),
            )
            .boxed(),
        )
    };

    (values.boxed(), exponents, patches)
}

#[allow(dead_code)]
pub fn alp_decode(parray: &PrimitiveArray, exp: ALPExponents) -> PrimitiveArray {
    match parray.ptype() {
        PType::I32 => PrimitiveArray::from_vec_in(
            alp::decode::<f32>(parray.buffer().typed_data::<i32>(), exp).unwrap(),
        ),
        PType::I64 => PrimitiveArray::from_vec_in(
            alp::decode::<f64>(parray.buffer().typed_data::<i64>(), exp).unwrap(),
        ),
        _ => panic!("Unsupported ptype"),
    }
}