use super::{GlweCompressionKey, GlweDecompressionKey};
use crate::core_crypto::prelude::compressed_modulus_switched_glwe_ciphertext::CompressedModulusSwitchedGlweCiphertext;
use crate::core_crypto::prelude::{
    extract_lwe_sample_from_glwe_ciphertext,
    par_keyswitch_lwe_ciphertext_list_and_pack_in_glwe_ciphertext, GlweCiphertext, LweCiphertext,
    LweCiphertextList, MonomialDegree,
};
use crate::shortint::ciphertext::GlwePackedCiphertextList;
use crate::shortint::engine::ShortintEngine;
use crate::shortint::parameters::NoiseLevel;
use crate::shortint::server_key::{
    apply_programmable_bootstrap, generate_lookup_table, unchecked_scalar_mul_assign,
};
use crate::shortint::{Ciphertext, CiphertextModulus};
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;

impl GlweCompressionKey {
    pub fn pack_lwe_ciphertexts_into_glwes(
        &self,
        ciphertexts: &[Ciphertext],
    ) -> GlwePackedCiphertextList {
        let count = ciphertexts.len();

        let lwe_pksk = &self.packing_key_switching_key;

        let polynomial_size = lwe_pksk.output_polynomial_size();
        let ciphertext_modulus = lwe_pksk.ciphertext_modulus();
        let glwe_size = lwe_pksk.output_glwe_size();
        let lwe_size = lwe_pksk.input_key_lwe_dimension().to_lwe_size();

        let lwe_per_glwe = self.lwe_per_glwe;

        assert!(
            lwe_per_glwe <= polynomial_size.0,
            "Cannot pack more than polynomial_size(={}) elements per glwe, {} requested",
            polynomial_size.0,
            lwe_per_glwe,
        );

        let first_ct = &ciphertexts[0];

        let message_modulus = first_ct.message_modulus;
        let carry_modulus = first_ct.carry_modulus;
        let pbs_order = first_ct.pbs_order;

        assert!(message_modulus.0 <= carry_modulus.0);

        let glwe_ct_list: Vec<_> = ciphertexts
            .par_chunks(lwe_per_glwe)
            .map(|ct_list| {
                let mut list: Vec<_> = vec![];

                for ct in ct_list {
                    assert!(ct.carry_is_empty());

                    assert_eq!(
                        lwe_size,
                        ct.ct.lwe_size(),
                        "All ciphertexts do not have the same lwe size as the packing keyswitch key"
                    );

                    assert_eq!(
                        message_modulus, ct.message_modulus,
                        "All ciphertexts do not have the same message modulus"
                    );
                    assert_eq!(
                        carry_modulus, ct.carry_modulus,
                        "All ciphertexts do not have the same carry modulus"
                    );
                    assert_eq!(
                        pbs_order, ct.pbs_order,
                        "All ciphertexts do not have the same pbs order"
                    );

                    let mut ct = ct.clone();

                    unchecked_scalar_mul_assign(&mut ct, message_modulus.0 as u8);

                    list.extend(ct.ct.as_view().into_container());
                }

                let list = LweCiphertextList::from_container(list, lwe_size, ciphertext_modulus);

                let bodies_count = ct_list.len();

                let mut out =
                    GlweCiphertext::new(0, glwe_size, polynomial_size, ciphertext_modulus);

                par_keyswitch_lwe_ciphertext_list_and_pack_in_glwe_ciphertext(
                    lwe_pksk, &list, &mut out,
                );

                CompressedModulusSwitchedGlweCiphertext::compress(
                    &out,
                    self.polynomial_size.to_blind_rotation_input_modulus_log(),
                    bodies_count,
                )
            })
            .collect();

        GlwePackedCiphertextList {
            compressed_modulus_switched_lwe_ciphertext: glwe_ct_list,
            message_modulus,
            carry_modulus,
            pbs_order,
            lwe_per_glwe,
            count,
            glwe_dimension: glwe_size.to_glwe_dimension(),
            polynomial_size,
            ciphertext_modulus,
        }
    }
}

impl GlweDecompressionKey {
    pub fn unpack(&self, packed: &GlwePackedCiphertextList, index: usize) -> Ciphertext {
        let carry_extract = generate_lookup_table(
            self.out_glwe_size(),
            self.out_polynomial_size(),
            packed.ciphertext_modulus,
            packed.message_modulus,
            packed.carry_modulus,
            |x| x / packed.message_modulus.0 as u64,
        );
        assert!(
            index < packed.count,
            "asked for element at index {} , but there are only {} elements",
            index,
            packed.count
        );

        let polynomial_size = packed.polynomial_size;
        let ciphertext_modulus = packed.ciphertext_modulus;

        let lwe_size = packed
            .glwe_dimension
            .to_equivalent_lwe_dimension(polynomial_size)
            .to_lwe_size();

        let glwe_index = index / packed.lwe_per_glwe;

        let packed_glwe = packed.compressed_modulus_switched_lwe_ciphertext[glwe_index].extract();

        let monomial_degree = MonomialDegree(index - glwe_index * packed.lwe_per_glwe);

        let mut intermediate_lwe = LweCiphertext::new(0, lwe_size, ciphertext_modulus);

        extract_lwe_sample_from_glwe_ciphertext(
            &packed_glwe,
            &mut intermediate_lwe,
            monomial_degree,
        );

        let mut output_br = LweCiphertext::new(
            0,
            self.blind_rotate_key
                .glwe_size()
                .to_glwe_dimension()
                .to_equivalent_lwe_dimension(self.blind_rotate_key.polynomial_size())
                .to_lwe_size(),
            ciphertext_modulus,
        );

        ShortintEngine::with_thread_local_mut(|engine| {
            let (_ciphertext_buffers, buffers) = engine.get_buffers_no_sk(
                self.blind_rotate_key.input_lwe_dimension(),
                self.blind_rotate_key.output_lwe_dimension(),
                CiphertextModulus::new_native(),
            );

            apply_programmable_bootstrap(
                &self.blind_rotate_key,
                &intermediate_lwe,
                &mut output_br,
                carry_extract.acc.clone(),
                buffers,
            );
        });

        Ciphertext::new(
            output_br,
            carry_extract.degree,
            NoiseLevel::NOMINAL,
            packed.message_modulus,
            packed.carry_modulus,
            packed.pbs_order,
        )
    }
}

#[cfg(test)]
mod test {
    use super::super::COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS;
    use super::*;
    use crate::shortint::ClientKey;
    use rayon::iter::IntoParallelIterator;

    #[test]
    fn test_packing() {
        use crate::shortint::gen_keys;
        use crate::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS;

        // Generate the client key and the server key:
        let (cks, _sks) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);

        let private_compression_key: crate::shortint::glwe_compression::GlweCompressionPrivateKeys =
            cks.new_compression_private_key(COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS);

        let (compression_key, decompression_key) =
            cks.new_compression_decompression_keys(&private_compression_key);

        for number_to_pack in [1, 128] {
            let f = |x| (x + 1) % 4;

            test_packing_(
                &compression_key,
                &decompression_key,
                &cks,
                f,
                number_to_pack,
            );
        }
    }

    fn test_packing_(
        comp_key: &GlweCompressionKey,
        decomp_key: &GlweDecompressionKey,
        cks: &ClientKey,
        f: impl Fn(u64) -> u64 + Sync,
        number_to_pack: usize,
    ) {
        let ct: Vec<_> = (0..number_to_pack)
            .map(|i| cks.encrypt(f(i as u64)))
            .collect();

        let packed = comp_key.pack_lwe_ciphertexts_into_glwes(&ct);

        (0..number_to_pack).into_par_iter().for_each(|i| {
            let unpacked = decomp_key.unpack(&packed, i);

            let res = cks.decrypt_message_and_carry(&unpacked);

            assert_eq!(f(i as u64), res);
        });
    }
}
