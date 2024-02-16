use std::fs::{create_dir_all, File};
use std::path::Path;

use arrow_array::RecordBatchReader;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rand::distributions::{Alphanumeric, Uniform};
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};

use enc::array::chunked::ChunkedArray;
use enc::array::primitive::PrimitiveArray;
use enc::array::varbin::VarBinArray;
use enc::array::{Array, ArrayRef};
use enc::compress::CompressCtx;
use enc::dtype::DType;
use enc::error::{EncError, EncResult};

use enc_bench::enumerate_arrays;

fn download_taxi_data() -> &'static Path {
    let download_path = Path::new("../../pyspiral/bench/.data/https-d37ci6vzurychx-cloudfront-net-trip-data-yellow-tripdata-2023-11.parquet");
    if download_path.exists() {
        return download_path;
    }

    create_dir_all(download_path.parent().unwrap()).unwrap();
    let mut download_file = File::create(download_path).unwrap();
    reqwest::blocking::get(
        "https://d37ci6vzurychx.cloudfront.net/trip-data/yellow_tripdata_2023-11.parquet",
    )
    .unwrap()
    .copy_to(&mut download_file)
    .unwrap();

    download_path
}

fn compress(array: ArrayRef) -> usize {
    CompressCtx::default()
        .compress(array.as_ref(), None)
        .nbytes()
}

fn enc_compress(c: &mut Criterion) {
    enumerate_arrays();

    let file = File::open(download_taxi_data()).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .with_batch_size(128_000)
        .build()
        .unwrap();

    let schema = reader.schema();
    let dtype: DType = schema.try_into().unwrap();
    let chunks = reader
        .map(|batch_result| batch_result.map_err(EncError::from))
        .map(|batch| batch.map(|b| b.into()))
        .collect::<EncResult<Vec<ArrayRef>>>()
        .unwrap();
    let chunked = ChunkedArray::new(chunks, dtype);
    println!(
        "{} rows in {} chunks",
        chunked.len(),
        chunked.chunks().len()
    );
    let array = chunked.boxed();

    c.bench_function("enc.compress", |b| {
        b.iter(|| black_box(compress(array.clone())))
    });
}

fn gen_primitive_dict(len: usize, uniqueness: f64) -> PrimitiveArray {
    let mut rng = thread_rng();
    let value_range = len as f64 * uniqueness;
    let range = Uniform::new(-(value_range / 2.0) as i32, (value_range / 2.0) as i32);
    let data: Vec<i32> = (0..len).map(|_| rng.sample(range)).collect();

    PrimitiveArray::from_vec(data)
}

fn gen_varbin_dict(len: usize, uniqueness: f64) -> VarBinArray {
    let mut rng = thread_rng();
    let uniq_cnt = (len as f64 * uniqueness) as usize;
    let dict: Vec<String> = (0..uniq_cnt)
        .map(|_| {
            (&mut rng)
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect()
        })
        .collect();
    let words: Vec<&str> = (0..len)
        .map(|_| dict.choose(&mut rng).unwrap().as_str())
        .collect();
    VarBinArray::from(words)
}

fn dict_encode_primitive(arr: &PrimitiveArray) -> usize {
    let (codes, values) = enc_dict::dict_encode_primitive(arr);
    (codes.nbytes() + values.nbytes()) / arr.nbytes()
}

fn dict_encode_varbin(arr: &VarBinArray) -> usize {
    let (codes, values) = enc_dict::dict_encode_varbin(arr);
    (codes.nbytes() + values.nbytes()) / arr.nbytes()
}

fn dict_encode(c: &mut Criterion) {
    let primitive_arr = gen_primitive_dict(1_000_000, 0.05);
    let varbin_arr = gen_varbin_dict(1_000_000, 0.05);

    c.bench_function("enc.dict_encode_primitives", |b| {
        b.iter(|| black_box(dict_encode_primitive(&primitive_arr)));
    });
    c.bench_function("enc.dict_encode_varbin", |b| {
        b.iter(|| black_box(dict_encode_varbin(&varbin_arr)));
    });
}

criterion_group!(benches, enc_compress, dict_encode);
criterion_main!(benches);