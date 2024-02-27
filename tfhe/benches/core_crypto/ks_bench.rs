#[path = "../utilities.rs"]
mod utilities;

use crate::utilities::{write_to_json, CryptoParametersRecord, OperatorType};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::Serialize;
use std::string::ToString;
use tfhe::boolean::prelude::*;
use tfhe::core_crypto::prelude::*;
use tfhe::keycache::NamedParam;
#[cfg(not(feature = "gpu"))]
use tfhe::shortint::parameters::{
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_3_CARRY_3_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_3_CARRY_3_GROUP_3_KS_PBS,
};
#[cfg(feature = "gpu")]
use tfhe::shortint::parameters::{
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS,
};
use tfhe::shortint::prelude::*;
use tfhe::shortint::{MultiBitPBSParameters, PBSParameters};

const SHORTINT_BENCH_PARAMS: [ClassicPBSParameters; 4] = [
    PARAM_MESSAGE_1_CARRY_1_KS_PBS,
    PARAM_MESSAGE_2_CARRY_2_KS_PBS,
    PARAM_MESSAGE_3_CARRY_3_KS_PBS,
    PARAM_MESSAGE_4_CARRY_4_KS_PBS,
];

#[cfg(not(feature = "gpu"))]
const SHORTINT_MULTI_BIT_BENCH_PARAMS: [MultiBitPBSParameters; 6] = [
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_3_CARRY_3_GROUP_2_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_3_CARRY_3_GROUP_3_KS_PBS,
];

#[cfg(feature = "gpu")]
const SHORTINT_MULTI_BIT_BENCH_PARAMS: [MultiBitPBSParameters; 2] = [
    PARAM_MULTI_BIT_MESSAGE_1_CARRY_1_GROUP_3_KS_PBS,
    PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS,
];

const BOOLEAN_BENCH_PARAMS: [(&str, BooleanParameters); 2] = [
    ("BOOLEAN_DEFAULT_PARAMS", DEFAULT_PARAMETERS),
    (
        "BOOLEAN_TFHE_LIB_PARAMS",
        PARAMETERS_ERROR_PROB_2_POW_MINUS_165,
    ),
];

fn benchmark_parameters<Scalar: UnsignedInteger>() -> Vec<(String, CryptoParametersRecord<Scalar>)>
{
    if Scalar::BITS == 64 {
        let classic = SHORTINT_BENCH_PARAMS
            .iter()
            .map(|params| {
                (
                    params.name(),
                    <ClassicPBSParameters as Into<PBSParameters>>::into(*params)
                        .to_owned()
                        .into(),
                )
            })
            .collect::<Vec<(String, CryptoParametersRecord<Scalar>)>>();
        let multi_bit = SHORTINT_MULTI_BIT_BENCH_PARAMS
            .iter()
            .map(|params| {
                (
                    params.name(),
                    <MultiBitPBSParameters as Into<PBSParameters>>::into(*params)
                        .to_owned()
                        .into(),
                )
            })
            .collect();
        [classic, multi_bit].concat()
    } else if Scalar::BITS == 32 {
        BOOLEAN_BENCH_PARAMS
            .iter()
            .map(|(name, params)| (name.to_string(), params.to_owned().into()))
            .collect()
    } else {
        vec![]
    }
}

fn keyswitch<Scalar: UnsignedTorus + CastInto<usize> + Serialize>(criterion: &mut Criterion) {
    let bench_name = "core_crypto::keyswitch";
    let mut bench_group = criterion.benchmark_group(bench_name);

    // Create the PRNG
    let mut seeder = new_seeder();
    let seeder = seeder.as_mut();
    let mut encryption_generator =
        EncryptionRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed(), seeder);
    let mut secret_generator =
        SecretRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed());

    for (name, params) in benchmark_parameters::<Scalar>().iter() {
        let lwe_dimension = params.lwe_dimension.unwrap();
        let lwe_modular_std_dev = params.lwe_modular_std_dev.unwrap();
        let glwe_dimension = params.glwe_dimension.unwrap();
        let polynomial_size = params.polynomial_size.unwrap();
        let ks_decomp_base_log = params.ks_base_log.unwrap();
        let ks_decomp_level_count = params.ks_level.unwrap();

        let lwe_sk =
            allocate_and_generate_new_binary_lwe_secret_key(lwe_dimension, &mut secret_generator);

        let glwe_sk = allocate_and_generate_new_binary_glwe_secret_key(
            glwe_dimension,
            polynomial_size,
            &mut secret_generator,
        );
        let big_lwe_sk = glwe_sk.into_lwe_secret_key();
        let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
            &big_lwe_sk,
            &lwe_sk,
            ks_decomp_base_log,
            ks_decomp_level_count,
            lwe_modular_std_dev,
            tfhe::core_crypto::prelude::CiphertextModulus::new_native(),
            &mut encryption_generator,
        );

        let ct = allocate_and_encrypt_new_lwe_ciphertext(
            &big_lwe_sk,
            Plaintext(Scalar::ONE),
            lwe_modular_std_dev,
            tfhe::core_crypto::prelude::CiphertextModulus::new_native(),
            &mut encryption_generator,
        );

        let mut output_ct = LweCiphertext::new(
            Scalar::ZERO,
            lwe_sk.lwe_dimension().to_lwe_size(),
            tfhe::core_crypto::prelude::CiphertextModulus::new_native(),
        );

        let id = format!("{bench_name}_{name}");
        {
            bench_group.bench_function(&id, |b| {
                b.iter(|| {
                    keyswitch_lwe_ciphertext(&ksk_big_to_small, &ct, &mut output_ct);
                    black_box(&mut output_ct);
                })
            });
        }
        let bit_size = (params.message_modulus.unwrap_or(2) as u32).ilog2();
        write_to_json(
            &id,
            *params,
            name,
            "ks",
            &OperatorType::Atomic,
            bit_size,
            vec![bit_size],
        );
    }
}

// TODO générifier la fonction à la manière des benchs integer et créer une macro pour lancer
//  la version séquentielle et la version parallèle sans dupliquer l'implèm.
fn private_functional_packing_keyswitch<Scalar: UnsignedTorus + CastInto<usize> + Serialize>(
    criterion: &mut Criterion,
) {
    let bench_name = "core_crypto::pfpks";
    let mut bench_group = criterion.benchmark_group(bench_name);

    // Create the PRNG
    let mut seeder = new_seeder();
    let seeder = seeder.as_mut();
    let mut encryption_generator =
        EncryptionRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed(), seeder);
    let mut secret_generator =
        SecretRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed());

    for (name, params) in PFPKS_BENCH_PARAMS.iter() {
        let params: CryptoParametersRecord<Scalar> = (*params).into();

        let lwe_dimension = params.lwe_dimension.unwrap();
        let lwe_modular_std_dev = params.lwe_modular_std_dev.unwrap();
        let glwe_dimension = params.glwe_dimension.unwrap();
        let glwe_size = glwe_dimension.to_glwe_size();
        let output_polynomial_size = params.polynomial_size.unwrap();
        let decomp_base_log = params.pfks_base_log.unwrap();
        let decomp_level_count = params.pfks_level.unwrap();
        let ciphertext_modulus = params.ciphertext_modulus.unwrap();

        let mut lwe_pfpksk = LwePrivateFunctionalPackingKeyswitchKey::new(
            Scalar::ZERO,
            decomp_base_log,
            decomp_level_count,
            lwe_dimension,
            glwe_size,
            output_polynomial_size,
            ciphertext_modulus,
        );

        let input_lwe_secret_key =
            allocate_and_generate_new_binary_lwe_secret_key(lwe_dimension, &mut secret_generator);

        let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
            glwe_dimension,
            output_polynomial_size,
            &mut secret_generator,
        );

        let polynomial = Polynomial::new(Scalar::ZERO, output_polynomial_size);

        par_generate_lwe_private_functional_packing_keyswitch_key(
            &input_lwe_secret_key,
            &output_glwe_secret_key,
            &mut lwe_pfpksk,
            lwe_modular_std_dev,
            &mut encryption_generator,
            UnsignedInteger::wrapping_neg,
            &polynomial,
        );

        let lwe_list = LweCiphertextList::new(
            Scalar::ZERO,
            lwe_dimension.to_lwe_size(),
            LweCiphertextCount(output_polynomial_size.0),  // FIXME Replace with higher number (128) when params available
            ciphertext_modulus,
        );

        let mut output_glwe = GlweCiphertext::new(
            Scalar::ZERO,
            glwe_size,
            output_polynomial_size,
            ciphertext_modulus,
        );

        // TODO parallel version to benchmaks afterwards
        // let mut output_glwe_parallel = GlweCiphertext::new(
        //     Scalar::ZERO,
        //     glwe_dimension,
        //     output_polynomial_size,
        //     ciphertext_modulus,
        // );
        //
        // par_private_functional_keyswitch_lwe_ciphertext_into_glwe_ciphertext(
        //     &lwe_pfpksk,
        //     &mut output_glwe_parallel,
        //     &random_lwe,
        // );

        for sub_list_len in [8, ] {
            let sub_list = lwe_list.get_sub(0..(sub_list_len -1));
            let sub_list_size = sub_list.lwe_ciphertext_count().0;

            let id = format!("{bench_name}_{name}_list_size_{sub_list_size}");
            {
                bench_group.bench_function(&id, |b| {
                    b.iter(|| {
                        private_functional_keyswitch_lwe_ciphertext_list_and_pack_in_glwe_ciphertext(
                            &lwe_pfpksk,
                            &mut output_glwe,
                            &sub_list,
                        );
                        black_box(&mut output_glwe);
                    })
                });
            }
            let bit_size = (params.message_modulus.unwrap_or(2) as u32).ilog2();
            write_to_json(
                &id,
                params,
                *name,
                "pfpks",
                &OperatorType::Atomic,
                bit_size,
                vec![bit_size],
            );
        }
    }
}

#[cfg(feature = "gpu")]
mod cuda {
    use crate::benchmark_parameters;
    use crate::utilities::{write_to_json, OperatorType};
    use criterion::{black_box, criterion_group, Criterion};
    use serde::Serialize;
    use tfhe::core_crypto::gpu::lwe_ciphertext_list::CudaLweCiphertextList;
    use tfhe::core_crypto::gpu::lwe_keyswitch_key::CudaLweKeyswitchKey;
    use tfhe::core_crypto::gpu::{cuda_keyswitch_lwe_ciphertext, CudaDevice, CudaStream};
    use tfhe::core_crypto::prelude::*;

    fn cuda_keyswitch<Scalar: UnsignedTorus + CastInto<usize> + Serialize>(
        criterion: &mut Criterion,
    ) {
        let bench_name = "core_crypto::cuda::keyswitch";
        let mut bench_group = criterion.benchmark_group(bench_name);

        // Create the PRNG
        let mut seeder = new_seeder();
        let seeder = seeder.as_mut();
        let mut encryption_generator =
            EncryptionRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed(), seeder);
        let mut secret_generator =
            SecretRandomGenerator::<ActivatedRandomGenerator>::new(seeder.seed());

        let gpu_index = 0;
        let device = CudaDevice::new(gpu_index);
        let stream = CudaStream::new_unchecked(device);

        for (name, params) in benchmark_parameters::<Scalar>().iter() {
            let lwe_dimension = params.lwe_dimension.unwrap();
            let lwe_modular_std_dev = params.lwe_modular_std_dev.unwrap();
            let glwe_dimension = params.glwe_dimension.unwrap();
            let polynomial_size = params.polynomial_size.unwrap();
            let ks_decomp_base_log = params.ks_base_log.unwrap();
            let ks_decomp_level_count = params.ks_level.unwrap();

            let lwe_sk = allocate_and_generate_new_binary_lwe_secret_key(
                lwe_dimension,
                &mut secret_generator,
            );

            let glwe_sk = allocate_and_generate_new_binary_glwe_secret_key(
                glwe_dimension,
                polynomial_size,
                &mut secret_generator,
            );
            let big_lwe_sk = glwe_sk.into_lwe_secret_key();
            let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
                &big_lwe_sk,
                &lwe_sk,
                ks_decomp_base_log,
                ks_decomp_level_count,
                lwe_modular_std_dev,
                CiphertextModulus::new_native(),
                &mut encryption_generator,
            );
            let ksk_big_to_small_gpu =
                CudaLweKeyswitchKey::from_lwe_keyswitch_key(&ksk_big_to_small, &stream);

            let ct = allocate_and_encrypt_new_lwe_ciphertext(
                &big_lwe_sk,
                Plaintext(Scalar::ONE),
                lwe_modular_std_dev,
                CiphertextModulus::new_native(),
                &mut encryption_generator,
            );
            let mut ct_gpu = CudaLweCiphertextList::from_lwe_ciphertext(&ct, &stream);

            let output_ct = LweCiphertext::new(
                Scalar::ZERO,
                lwe_sk.lwe_dimension().to_lwe_size(),
                CiphertextModulus::new_native(),
            );
            let mut output_ct_gpu = CudaLweCiphertextList::from_lwe_ciphertext(&output_ct, &stream);

            let h_indexes = &[Scalar::ZERO];
            let mut d_input_indexes = unsafe { stream.malloc_async::<Scalar>(1u32) };
            let mut d_output_indexes = unsafe { stream.malloc_async::<Scalar>(1u32) };
            unsafe {
                stream.copy_to_gpu_async(&mut d_input_indexes, h_indexes.as_ref());
                stream.copy_to_gpu_async(&mut d_output_indexes, h_indexes.as_ref());
            }
            stream.synchronize();

            let id = format!("{bench_name}_{name}");
            {
                bench_group.bench_function(&id, |b| {
                    b.iter(|| {
                        cuda_keyswitch_lwe_ciphertext(
                            &ksk_big_to_small_gpu,
                            &ct_gpu,
                            &mut output_ct_gpu,
                            &d_input_indexes,
                            &d_output_indexes,
                            &stream,
                        );
                        black_box(&mut ct_gpu);
                    })
                });
            }
            let bit_size = (params.message_modulus.unwrap_or(2) as u32).ilog2();
            write_to_json(
                &id,
                *params,
                name,
                "ks",
                &OperatorType::Atomic,
                bit_size,
                vec![bit_size],
            );
        }
    }
    criterion_group!(
        name = cuda_keyswitch_group;
        config = Criterion::default().sample_size(2000);
        targets = cuda_keyswitch::<u64>
    );
}

#[cfg(feature = "gpu")]
use cuda::cuda_keyswitch_group;

criterion_group!(
    name = keyswitch_group;
    config = Criterion::default().sample_size(2000);
    targets = keyswitch::<u64>, keyswitch::<u32>
);

criterion_group!(
    name = packing_keyswitch_group;
    config = Criterion::default().sample_size(2000);
    targets = private_functional_packing_keyswitch::<u64>
);

#[cfg(not(feature = "gpu"))]
// criterion_main!(keyswitch_group, packing_keyswitch_group);
criterion_main!(packing_keyswitch_group);
#[cfg(feature = "gpu")]
criterion_main!(cuda_keyswitch_group);

// --------------------------------------------------------
// Special benchmark parameters, no guarantees that they are secure
// --------------------------------------------------------
use tfhe::shortint::WopbsParameters;

const PFPKS_BENCH_PARAMS: [(&str, WopbsParameters); 1] = [
    // (
    //     stringify!(ID_7_RADIX_16_BITS_16_BLOCKS_WOPBS),
    //     ID_7_RADIX_16_BITS_16_BLOCKS_WOPBS,
    // ),
    (
        stringify!(ID_8_RADIX_16_BITS_8_BLOCKS_WOPBS),
        ID_8_RADIX_16_BITS_8_BLOCKS_WOPBS,
    ),
    // (
    //     stringify!(ID_9_CRT_16_BITS_5_BLOCKS_WOPBS),
    //     ID_9_CRT_16_BITS_5_BLOCKS_WOPBS,
    // ),
    // (
    //     stringify!(ID_10_NATIF_CRT_16_BITS_5_BLOCKS_WOPBS),
    //     ID_10_NATIF_CRT_16_BITS_5_BLOCKS_WOPBS,
    // ),
    // (
    //     stringify!(ID_11_NATIF_CRT_32_BITS_6_BLOCKS_WOPBS),
    //     ID_11_NATIF_CRT_32_BITS_6_BLOCKS_WOPBS,
    // ),
    // (
    //     stringify!(ID_11_BIS_NATIF_CRT_32_BITS_8_BLOCKS_WOPBS),
    //     ID_11_BIS_NATIF_CRT_32_BITS_8_BLOCKS_WOPBS,
    // ),
];

// pub const ID_7_RADIX_16_BITS_16_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
//     lwe_dimension: LweDimension(549),
//     glwe_dimension: GlweDimension(2),
//     polynomial_size: PolynomialSize(1024),
//     lwe_modular_std_dev: StandardDev(0.0003177104139262535),
//     glwe_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
//     pbs_base_log: DecompositionBaseLog(12),
//     pbs_level: DecompositionLevelCount(3),
//     ks_level: DecompositionLevelCount(5),
//     ks_base_log: DecompositionBaseLog(2),
//     pfks_level: DecompositionLevelCount(2),
//     pfks_base_log: DecompositionBaseLog(17),
//     pfks_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
//     cbs_level: DecompositionLevelCount(1),
//     cbs_base_log: DecompositionBaseLog(13),
//     message_modulus: MessageModulus(2),
//     carry_modulus: CarryModulus(2),
//     ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
//     encryption_key_choice: EncryptionKeyChoice::Big,
// };
pub const ID_8_RADIX_16_BITS_8_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
    lwe_dimension: LweDimension(534),
    glwe_dimension: GlweDimension(2),
    polynomial_size: PolynomialSize(1024),
    lwe_modular_std_dev: StandardDev(0.0004192214045106218),
    glwe_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
    pbs_base_log: DecompositionBaseLog(12),
    pbs_level: DecompositionLevelCount(3),
    ks_level: DecompositionLevelCount(5),
    ks_base_log: DecompositionBaseLog(2),
    pfks_level: DecompositionLevelCount(2),
    pfks_base_log: DecompositionBaseLog(17),
    pfks_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
    cbs_level: DecompositionLevelCount(2),
    cbs_base_log: DecompositionBaseLog(9),
    message_modulus: MessageModulus(4),
    carry_modulus: CarryModulus(4),
    ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
    encryption_key_choice: EncryptionKeyChoice::Big,
};
// pub const ID_9_CRT_16_BITS_5_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
//     lwe_dimension: LweDimension(538),
//     glwe_dimension: GlweDimension(4),
//     polynomial_size: PolynomialSize(1024),
//     lwe_modular_std_dev: StandardDev(0.00038844554870845634),
//     glwe_modular_std_dev: StandardDev(0.0000000000000000002168404344971009),
//     pbs_base_log: DecompositionBaseLog(4),
//     pbs_level: DecompositionLevelCount(11),
//     ks_level: DecompositionLevelCount(10),
//     ks_base_log: DecompositionBaseLog(1),
//     pfks_level: DecompositionLevelCount(2),
//     pfks_base_log: DecompositionBaseLog(20),
//     pfks_modular_std_dev: StandardDev(0.0000000000000000002168404344971009),
//     cbs_level: DecompositionLevelCount(4),
//     cbs_base_log: DecompositionBaseLog(7),
//     message_modulus: MessageModulus(16),
//     carry_modulus: CarryModulus(4),
//     ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
//     encryption_key_choice: EncryptionKeyChoice::Big,
// };
// pub const ID_10_NATIF_CRT_16_BITS_5_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
//     lwe_dimension: LweDimension(696),
//     glwe_dimension: GlweDimension(2),
//     polynomial_size: PolynomialSize(1024),
//     lwe_modular_std_dev: StandardDev(0.00002113509320237618),
//     glwe_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
//     pbs_base_log: DecompositionBaseLog(9),
//     pbs_level: DecompositionLevelCount(4),
//     ks_level: DecompositionLevelCount(7),
//     ks_base_log: DecompositionBaseLog(2),
//     pfks_level: DecompositionLevelCount(2),
//     pfks_base_log: DecompositionBaseLog(17),
//     pfks_modular_std_dev: StandardDev(0.0000000000000003162026630747649),
//     cbs_level: DecompositionLevelCount(3),
//     cbs_base_log: DecompositionBaseLog(7),
//     message_modulus: MessageModulus(16),
//     carry_modulus: CarryModulus(1),
//     ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
//     encryption_key_choice: EncryptionKeyChoice::Big,
// };
// pub const ID_11_NATIF_CRT_32_BITS_6_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
//     lwe_dimension: LweDimension(791),
//     glwe_dimension: GlweDimension(1),
//     polynomial_size: PolynomialSize(4096),
//     lwe_modular_std_dev: StandardDev(0.000003659302213002263),
//     glwe_modular_std_dev: StandardDev(0.0000000000000000002168404344971009),
//     pbs_base_log: DecompositionBaseLog(3),
//     pbs_level: DecompositionLevelCount(14),
//     ks_level: DecompositionLevelCount(16),
//     ks_base_log: DecompositionBaseLog(1),
//     pfks_level: DecompositionLevelCount(2),
//     pfks_base_log: DecompositionBaseLog(20),
//     pfks_modular_std_dev: StandardDev(0.0000000000000000002168404344971009),
//     cbs_level: DecompositionLevelCount(5),
//     cbs_base_log: DecompositionBaseLog(5),
//     message_modulus: MessageModulus(64),
//     carry_modulus: CarryModulus(1),
//     ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
//     encryption_key_choice: EncryptionKeyChoice::Big,
// };
// pub const ID_11_BIS_NATIF_CRT_32_BITS_8_BLOCKS_WOPBS: WopbsParameters = WopbsParameters {
//     lwe_dimension: LweDimension(781),
//     glwe_dimension: GlweDimension(1),
//     polynomial_size: PolynomialSize(2048),
//     lwe_modular_std_dev: StandardDev(0.0000044043577651404615),
//     glwe_modular_std_dev: StandardDev(0.0000000000000003152931493498455),
//     pbs_base_log: DecompositionBaseLog(5),
//     pbs_level: DecompositionLevelCount(8),
//     ks_level: DecompositionLevelCount(16),
//     ks_base_log: DecompositionBaseLog(1),
//     pfks_level: DecompositionLevelCount(3),
//     pfks_base_log: DecompositionBaseLog(13),
//     pfks_modular_std_dev: StandardDev(0.0000000000000003152931493498455),
//     cbs_level: DecompositionLevelCount(4),
//     cbs_base_log: DecompositionBaseLog(6),
//     message_modulus: MessageModulus(32),
//     carry_modulus: CarryModulus(1),
//     ciphertext_modulus: tfhe::shortint::CiphertextModulus::new_native(),
//     encryption_key_choice: EncryptionKeyChoice::Big,
// };
