use frame_support::assert_noop;
use hex_literal::hex;
use sp_core::{H160, U256};

use precompile_utils::data::EvmDataWriter;
use precompile_utils::testing::*;

use crate::currencies::Action;
use crate::mock::*;

fn precompiles() -> Precompiles<Runtime> {
	PrecompilesValue::get()
}

#[test]
fn handles_invalid_currency_id() {
	ExtBuilder::default().build().execute_with(|| {
		precompiles()
			.prepare_test(
				H160(hex!("0000000000000000000500000000000000000000")),
				H160(hex!("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b")),
				EvmDataWriter::new_with_selector(Action::TotalSupply).build(),
			)
			.expect_cost(0)
			.expect_no_logs()
			.execute_returns(EvmDataWriter::new().write(U256::from(3500u64)).build());
	});
}
/*

   #[test]
   fn total_supply_works() {
		ExtBuilder::default().build().execute_with(|| {
			precompiles()
				.prepare_test(
					neer_evm_address(),
					alice_evm_addr(),
					EvmDataWriter::new_with_selector(Action::TotalSupply).build(),
				)
				.expect_cost(0)
				.expect_no_logs()
				.execute_returns(EvmDataWriter::new().write(U256::from(3500u64)).build());
		});
   }

   #[test]
   fn balance_of_works() {
	    ExtBuilder::default().build().execute_with(|| {
			precompiles()
				.prepare_test(
					neer_evm_address(),
					alice_evm_addr(),
					EvmDataWriter::new_with_selector(Action::BalanceOf).build(),
				)
				.expect_cost(0)
				.expect_no_logs()
				.execute_returns(EvmDataWriter::new().write(U256::from(100000u64)).build());
		});
   }

   #[test]
   fn transfer_works() {
	   ExtBuilder::default().build().execute_with(|| {
			precompiles()
				.prepare_test(
					neer_evm_address(),
					alice_evm_addr(),
					EvmDataWriter::new_with_selector(Action::Transfer).build(),
				)
				.expect_cost(0)
				.expect_no_logs()
				.execute_returns(EvmDataWriter::new().write(U256::from(100000u64)).build());
		});
   }
*/
