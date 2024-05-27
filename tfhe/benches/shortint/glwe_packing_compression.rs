use criterion::{black_box, criterion_group, Criterion};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tfhe::shortint::glwe_compression::COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS;
use tfhe::shortint::prelude::*;

fn glwe_packing(c: &mut Criterion) {
    #[allow(clippy::single_element_loop)]
    for (name, param, comp_param, number_to_pack) in [(
        "1",
        PARAM_MESSAGE_2_CARRY_2_KS_PBS,
        COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS,
        256,
    )] {
        let bench_name = "shortint-glwe-packing";

        let mut bench_group = c.benchmark_group(bench_name);

        // Generate the client key and the server key:
        let (cks, _sks) = gen_keys(param);

        let private_compression_key = cks.new_compression_private_key(comp_param);

        let (compression_key, decompression_key) =
            cks.new_compression_decompression_keys(&private_compression_key);

        let ct: Vec<_> = (0..number_to_pack)
            .map(|_| cks.unchecked_encrypt(0))
            .collect();

        bench_group.bench_function(&format!("{name}_pack"), |b| {
            b.iter(|| {
                let packed = compression_key.pack_lwe_ciphertexts_into_glwes(&ct);

                _ = black_box(packed);
            })
        });

        let packed = compression_key.pack_lwe_ciphertexts_into_glwes(&ct);
        bench_group.bench_function(&format!("{name}_unpack"), |b| {
            b.iter(|| {
                (0..number_to_pack).into_par_iter().for_each(|i| {
                    let unpacked = decompression_key.unpack(&packed, i);

                    _ = black_box(unpacked);
                });
            })
        });

        bench_group.bench_function(&format!("{name}_pack_unpack"), |b| {
            b.iter(|| {
                let packed = compression_key.pack_lwe_ciphertexts_into_glwes(&ct);

                (0..number_to_pack).into_par_iter().for_each(|i| {
                    let unpacked = decompression_key.unpack(&packed, i);

                    _ = black_box(unpacked);
                });
            })
        });
    }
}

criterion_group!(glwe_packing2, glwe_packing);

fn main() {
    glwe_packing2();
    Criterion::default().configure_from_args().final_summary();
}
