use crate::c_api::high_level_api::booleans::FheBool;
use crate::c_api::high_level_api::integers::{
    FheInt10, FheInt12, FheInt128, FheInt14, FheInt16, FheInt160, FheInt2, FheInt256, FheInt32,
    FheInt4, FheInt6, FheInt64, FheInt8, FheUint10, FheUint12, FheUint128, FheUint14, FheUint16,
    FheUint160, FheUint2, FheUint256, FheUint32, FheUint4, FheUint6, FheUint64, FheUint8,
};
use crate::c_api::high_level_api::utils::{
    impl_destroy_on_type, impl_serialize_deserialize_on_type,
};
use crate::c_api::utils::{catch_panic, get_mut_checked, get_ref_checked};
use crate::shortint::MessageModulus;
use std::ffi::c_int;

pub struct GlwePackedCompressedCiphertextListBuilder(
    crate::high_level_api::GlwePackedCompressedCiphertextListBuilder,
);
impl_destroy_on_type!(GlwePackedCompressedCiphertextListBuilder);

pub struct GlwePackedCompressedCiphertextList(
    crate::high_level_api::GlwePackedCompressedCiphertextList,
);
impl_destroy_on_type!(GlwePackedCompressedCiphertextList);
impl_serialize_deserialize_on_type!(GlwePackedCompressedCiphertextList);

#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_builder_new(
    message_modulus: usize,
    builder: *mut *mut GlwePackedCompressedCiphertextListBuilder,
) -> c_int {
    catch_panic(|| {
        let inner = crate::high_level_api::GlwePackedCompressedCiphertextListBuilder::new(
            MessageModulus(message_modulus),
        );

        *builder = Box::into_raw(Box::new(GlwePackedCompressedCiphertextListBuilder(inner)));
    })
}

#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_builder_build(
    builder: *const GlwePackedCompressedCiphertextListBuilder,
    list: *mut *mut GlwePackedCompressedCiphertextList,
) -> c_int {
    catch_panic(|| {
        let builder: &GlwePackedCompressedCiphertextListBuilder = get_ref_checked(builder).unwrap();

        let inner = builder.0.build();

        *list = Box::into_raw(Box::new(GlwePackedCompressedCiphertextList(inner)));
    })
}

/// Pushes a boolean into the list
#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_builder_push_bool(
    builder: *mut GlwePackedCompressedCiphertextListBuilder,
    value: *const FheBool,
) -> c_int {
    catch_panic(|| {
        let builder = get_mut_checked(builder).unwrap();

        let value: &FheBool = get_ref_checked(value).unwrap();

        builder.0.push(value.0.clone());
    })
}

macro_rules! define_glwe_packed_compressed_ciphertext_list_builder_push_method {
    (
        unsigned: $($num_bits:literal),*
        $(,)?
    ) => {
        ::paste::paste!{
            $(
                #[doc = concat!("Pushes an unsigned integer of ", stringify!($num_bits), " bits to the list")]
                #[no_mangle]
                pub unsafe extern "C" fn [<glwe_packed_compressed_ciphertext_list_builder_push_u $num_bits>](
                    builder: *mut GlwePackedCompressedCiphertextListBuilder,
                    value: *const [<FheUint $num_bits>],
                ) -> c_int {
                    catch_panic(|| {
                        let builder = get_mut_checked(builder).unwrap();

                        let value: &[<FheUint $num_bits>] = get_ref_checked(value).unwrap();


                        builder.0.push(value.0.clone());
                    })
                }
            )*
        }
    };
    (
        signed: $($num_bits:literal),*
        $(,)?
    ) => {
        ::paste::paste!{
            $(
                #[doc = concat!("Pushes a signed integer of ", stringify!($num_bits), " bits to the list")]
                #[no_mangle]
                pub unsafe extern "C" fn [<glwe_packed_compressed_ciphertext_list_builder_push_i $num_bits>](
                    builder: *mut GlwePackedCompressedCiphertextListBuilder,
                    value: *const [<FheInt $num_bits>],
                ) -> c_int {
                    catch_panic(|| {
                        let builder = get_mut_checked(builder).unwrap();

                        let value: &[<FheInt $num_bits>] = get_ref_checked(value).unwrap();

                        builder.0.push(value.0.clone());
                    })
                }
            )*
        }
    };
}

define_glwe_packed_compressed_ciphertext_list_builder_push_method!(unsigned: 2, 4, 6, 8, 10, 12, 14, 16, 32, 64, 128, 160, 256);
define_glwe_packed_compressed_ciphertext_list_builder_push_method!(signed: 2, 4, 6, 8, 10, 12, 14, 16, 32, 64, 128, 160, 256);

#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_len(
    expander: *mut GlwePackedCompressedCiphertextList,
    out: *mut usize,
) -> c_int {
    catch_panic(|| {
        let expander = get_ref_checked(expander).unwrap();
        *out = expander.0.len();
    })
}

#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_get_kind_of(
    expander: *mut GlwePackedCompressedCiphertextList,
    index: usize,
    out: *mut super::FheTypes,
) -> c_int {
    let mut result = None;
    catch_panic(|| {
        let expander = get_ref_checked(expander).unwrap();
        result = expander.0.get_kind_of(index);
    });
    result.map_or(1, |r| {
        *out = r.into();
        0
    })
}

macro_rules! define_glwe_packed_ciphertext_list_get {
    (
        unsigned: $($num_bits:literal),*
        $(,)?
    ) => {
        ::paste::paste!(
            $(
                #[no_mangle]
                pub unsafe extern "C" fn [<glwe_packed_compressed_ciphertext_list_get_fhe_uint $num_bits>](
                    expander: *mut GlwePackedCompressedCiphertextList,
                    index: usize,
                    out: *mut *mut [<FheUint $num_bits>],
                ) -> c_int {
                    catch_panic(|| {
                        let expander = get_mut_checked(expander).unwrap();

                        let inner = expander.0.get(index).unwrap().unwrap();

                        *out = Box::into_raw(Box::new([<FheUint $num_bits>](inner)));
                    })
                }
            )*
        );
    };
    (
        signed: $($num_bits:literal),*
        $(,)?
    ) => {
        ::paste::paste!(
            $(
                #[no_mangle]
                pub unsafe extern "C" fn [<glwe_packed_compressed_ciphertext_list_get_fhe_int $num_bits>](
                    expander: *mut GlwePackedCompressedCiphertextList,
                    index: usize,
                    out: *mut *mut [<FheInt $num_bits>],
                ) -> c_int {
                    catch_panic(|| {
                        let expander = get_mut_checked(expander).unwrap();

                        let inner = expander.0.get(index).unwrap().unwrap();

                        *out = Box::into_raw(Box::new([<FheInt $num_bits>](inner)));
                    })
                }
            )*
        );
    }
}

define_glwe_packed_ciphertext_list_get!(unsigned: 2, 4, 6, 8, 10, 12, 14, 16, 32, 64, 128, 160, 256);
define_glwe_packed_ciphertext_list_get!(signed: 2, 4, 6, 8, 10, 12, 14, 16, 32, 64, 128, 160, 256);

#[no_mangle]
pub unsafe extern "C" fn glwe_packed_compressed_ciphertext_list_get_fhe_bool(
    expander: *mut GlwePackedCompressedCiphertextList,
    index: usize,
    out: *mut *mut FheBool,
) -> c_int {
    catch_panic(|| {
        let expander = get_mut_checked(expander).unwrap();

        let inner = expander.0.get(index).unwrap().unwrap();

        *out = Box::into_raw(Box::new(FheBool(inner)));
    })
}
