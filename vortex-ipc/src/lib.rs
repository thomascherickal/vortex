extern crate core;

use vortex_error::{vortex_err, VortexError};

pub const ALIGNMENT: usize = 64;

pub mod flatbuffers {
    pub use generated::vortex::*;

    #[allow(unused_imports)]
    #[allow(dead_code)]
    #[allow(non_camel_case_types)]
    #[allow(clippy::all)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/flatbuffers/message.rs"));
    }

    mod deps {
        pub mod array {
            pub use vortex::flatbuffers::array;
        }
        pub mod dtype {
            pub use vortex_schema::flatbuffers as dtype;
        }
    }
}

mod chunked;
pub mod iter;
mod messages;
pub mod reader;
pub mod writer;

pub(crate) const fn missing(field: &'static str) -> impl FnOnce() -> VortexError {
    move || vortex_err!(InvalidSerde: "missing field: {}", field)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    use vortex_array2::array::primitive::PrimitiveData;
    use vortex_array2::array::r#struct::StructData;
    use vortex_array2::{IntoArray, WithArray};
    use vortex_array2::{SerdeContext, ToArray, ToArrayData};

    use crate::iter::FallibleLendingIterator;
    use crate::reader::StreamReader;
    use crate::writer::StreamWriter;

    #[test]
    fn test_write_flatbuffer() {
        let col = PrimitiveData::from_vec(vec![0, 1, 2]).into_data();
        let nested_struct = StructData::try_new(
            vec![Arc::new("x".into()), Arc::new("y".into())],
            vec![col.clone(), col.clone()],
            3,
        )
        .unwrap()
        .into_data();

        let arr = StructData::try_new(
            vec![Arc::new("a".into()), Arc::new("b".into())],
            vec![col.clone(), nested_struct],
            3,
        )
        .unwrap()
        .into_array();

        // let batch = ColumnBatch::from(&arr.to_array());

        let mut cursor = Cursor::new(Vec::new());
        let ctx = SerdeContext::default();
        let mut writer = StreamWriter::try_new_unbuffered(&mut cursor, ctx).unwrap();
        arr.with_array(|a| writer.write(a)).unwrap();
        cursor.flush().unwrap();
        cursor.set_position(0);

        let mut ipc_reader = StreamReader::try_new_unbuffered(cursor).unwrap();

        // Read some number of arrays off the stream.
        while let Some(array_reader) = ipc_reader.next().unwrap() {
            let mut array_reader = array_reader;
            println!("DType: {:?}", array_reader.dtype());
            // Read some number of chunks from the stream.
            while let Some(chunk) = array_reader.next().unwrap() {
                println!("VIEW: {:?}", &chunk);
                let _data = chunk.to_array().to_array_data();
                // let taken = take(&chunk, &PrimitiveArray::from(vec![0, 3, 0, 1])).unwrap();
                // let taken = taken.as_primitive().typed_data::<i32>();
                // println!("Taken: {:?}", &taken);
            }
        }
    }
}