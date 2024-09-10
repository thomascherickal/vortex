use vortex::accessor::ArrayAccessor;
use vortex::array::{BoolArray, PrimitiveArray, VarBinArray};
use vortex::validity::{ArrayValidity, Validity};
use vortex::{Array, ArrayDType, IntoArray, IntoArrayVariant};
use vortex_dtype::{match_each_native_ptype, DType};

pub fn take_canonical_array(array: &Array, indices: &[usize]) -> Array {
    match array.dtype() {
        DType::Bool(_) => {
            let bool_array = array.clone().into_bool().unwrap();
            let vec_values = bool_array.boolean_buffer().iter().collect::<Vec<_>>();
            let vec_validity = bool_array
                .logical_validity()
                .into_array()
                .into_bool()
                .unwrap()
                .boolean_buffer()
                .iter()
                .collect::<Vec<_>>();
            BoolArray::from_vec(
                indices.iter().map(|i| vec_values[*i]).collect(),
                Validity::from(indices.iter().map(|i| vec_validity[*i]).collect::<Vec<_>>()),
            )
            .into_array()
        }
        DType::Primitive(p, _) => match_each_native_ptype!(p, |$P| {
            let primitive_array = array.clone().into_primitive().unwrap();
            let vec_values = primitive_array
                .maybe_null_slice::<$P>()
                .iter()
                .copied()
                .collect::<Vec<_>>();
            let vec_validity = primitive_array
                .logical_validity()
                .into_array()
                .into_bool()
                .unwrap()
                .boolean_buffer()
                .iter()
                .collect::<Vec<_>>();
            PrimitiveArray::from_vec(
                indices.iter().map(|i| vec_values[*i]).collect(),
                Validity::from(indices.iter().map(|i| vec_validity[*i]).collect::<Vec<_>>())
            )
            .into_array()
        }),
        DType::Utf8(_) | DType::Binary(_) => {
            let utf8 = array.clone().into_varbin().unwrap();
            let values = utf8
                .with_iterator(|iter| iter.map(|v| v.map(|u| u.to_vec())).collect::<Vec<_>>())
                .unwrap();
            VarBinArray::from_iter(
                indices.iter().map(|i| values[*i].clone()),
                array.dtype().clone(),
            )
            .into_array()
        }
        _ => unreachable!("Array::arbitrary will not generate other dtypes"),
    }
}