use crate::integer::server_key::radix_parallel::tests_cases_unsigned::{
    default_left_shift_test, default_right_shift_test, unchecked_left_shift_test,
    unchecked_right_shift_test,
};
use crate::integer::server_key::radix_parallel::tests_unsigned::CpuFunctionExecutor;
use crate::integer::tests::create_parametrized_test;
use crate::integer::ServerKey;
#[cfg(tarpaulin)]
use crate::shortint::parameters::coverage_parameters::*;
use crate::shortint::parameters::*;
use crate::shortint::parameters::classic::compact_pk::tuniform::p_fail_2_minus_40::ks_pbs::*;

create_parametrized_test!(
    integer_unchecked_left_shift {
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 3 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

create_parametrized_test!(
    integer_unchecked_right_shift{
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 3 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);
create_parametrized_test!(
    integer_left_shift{
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 3 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);
create_parametrized_test!(
    integer_right_shift{
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 3 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

fn integer_unchecked_right_shift<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::unchecked_right_shift_parallelized);
    unchecked_right_shift_test(param, executor);
}

fn integer_right_shift<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::right_shift_parallelized);
    default_right_shift_test(param, executor);
}

fn integer_unchecked_left_shift<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::unchecked_left_shift_parallelized);
    unchecked_left_shift_test(param, executor);
}

fn integer_left_shift<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::left_shift_parallelized);
    default_left_shift_test(param, executor);
}
