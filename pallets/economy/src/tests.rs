// This file is part of Metaverse.Network & Bit.Country.

// Copyright (C) 2020-2022 Metaverse.Network & Bit.Country .
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};

use sp_runtime::traits::BadOrigin;
use sp_std::default::Default;

use core_primitives::{Attributes, CollectionType, TokenType};
use mock::{RuntimeEvent, *};
use primitives::staking::Bond;
use primitives::GroupCollectionId;

use super::*;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountIdOf<Runtime> {
	[id; 32].into()
}

fn init_test_nft(owner: RuntimeOrigin, collection_id: GroupCollectionId, class_id: ClassId) {
	//Create group collection before class
	assert_ok!(NFTModule::create_group(RuntimeOrigin::root(), vec![1], vec![1]));

	assert_ok!(NFTModule::create_class(
		owner.clone(),
		vec![1],
		test_attributes(1),
		collection_id,
		TokenType::Transferable,
		CollectionType::Collectable,
		Perbill::from_percent(0u32),
		None
	));

	assert_ok!(NFTModule::mint(owner.clone(), class_id, vec![1], test_attributes(1), 1));
}

fn test_attributes(x: u8) -> Attributes {
	let mut attr: Attributes = BTreeMap::new();
	attr.insert(vec![x, x + 5], vec![x, x + 10]);
	attr
}

fn sub_account(nft_id: (ClassId, TokenId)) -> AccountId {
	<Runtime as Config>::EconomyTreasury::get().into_sub_account_truncating(nft_id)
}

fn get_mining_currency() -> FungibleTokenId {
	<Runtime as Config>::MiningCurrencyId::get()
}

#[test]
fn set_bit_power_exchange_rate_should_fail_bad_origin() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::set_bit_power_exchange_rate(RuntimeOrigin::signed(account(2)), EXCHANGE_RATE),
			BadOrigin
		);
	});
}

#[test]
fn set_bit_power_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::set_bit_power_exchange_rate(
			RuntimeOrigin::root(),
			EXCHANGE_RATE
		));

		assert_eq!(EconomyModule::get_bit_power_exchange_rate(), EXCHANGE_RATE);
	});
}

#[test]
fn stake_should_fail_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_EXCESS_BALANCE, None),
			Error::<Runtime>::InsufficientBalanceForStaking
		);
	});
}

#[test]
fn stake_should_fail_exit_queue_scheduled() {
	ExtBuilder::default().build().execute_with(|| {
		// Add account entry to ExitQueue
		ExitQueue::<Runtime>::insert(account(1), CURRENT_ROUND, STAKE_BALANCE);

		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BELOW_MINIMUM_BALANCE, None),
			Error::<Runtime>::ExitQueueAlreadyScheduled
		);
	});
}

#[test]
fn stake_should_fail_below_minimum() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BELOW_MINIMUM_BALANCE, None),
			Error::<Runtime>::StakeBelowMinimum
		);
	});
}

#[test]
fn stake_should_fail_for_non_existing_estate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE, Some(8u32.into())),
			Error::<Runtime>::StakeEstateDoesNotExist
		);
	});
}

#[test]
fn stake_should_fail_for_estate_not_owned_by_staker() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::stake(
				RuntimeOrigin::signed(account(1)),
				STAKE_BALANCE,
				Some(EXISTING_ESTATE_ID)
			),
			Error::<Runtime>::StakerNotEstateOwner
		);
	});
}
#[test]
fn stake_should_fail_for_estate_owned_by_staker_but_having_previously_staked_bond() {
	ExtBuilder::default().build().execute_with(|| {
		let prepopulated_bond = Bond {
			staker: account(2),
			amount: STAKE_BALANCE,
		};

		EstateStakingInfo::<Runtime>::insert(&OWNED_ESTATE_ID, prepopulated_bond);

		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE, Some(OWNED_ESTATE_ID)),
			Error::<Runtime>::PreviousOwnerStillStakesAtEstate
		);
	});
}

#[test]
fn stake_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::SelfStakedToEconomy101(account(1), STAKE_BALANCE))
		);

		assert_eq!(Balances::reserved_balance(account(1)), STAKE_BALANCE);

		assert_eq!(EconomyModule::get_staking_info(account(1)), STAKE_BALANCE);

		assert_eq!(EconomyModule::total_stake(), STAKE_BALANCE);
	});
}

#[test]
fn stake_should_work_for_estate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			Some(OWNED_ESTATE_ID)
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::EstateStakedToEconomy101(
				account(1),
				OWNED_ESTATE_ID,
				STAKE_BALANCE
			))
		);

		assert_eq!(Balances::reserved_balance(account(1)), STAKE_BALANCE);
		assert_eq!(
			EconomyModule::get_estate_staking_info(OWNED_ESTATE_ID).unwrap().staker,
			account(1)
		);
		assert_eq!(
			EconomyModule::get_estate_staking_info(OWNED_ESTATE_ID).unwrap().amount,
			STAKE_BALANCE
		);

		assert_eq!(EconomyModule::total_estate_stake(), STAKE_BALANCE);
	});
}

#[test]
fn stake_should_work_with_more_operations() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_ok!(EconomyModule::stake(RuntimeOrigin::signed(account(1)), 100, None));

		let total_staked_balance = STAKE_BALANCE + 100u128;

		assert_eq!(Balances::reserved_balance(account(1)), total_staked_balance);

		assert_eq!(EconomyModule::get_staking_info(account(1)), total_staked_balance);

		assert_eq!(EconomyModule::total_stake(), total_staked_balance);
	});
}

#[test]
fn unstake_should_fail_exceeds_staked_amount() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::unstake(RuntimeOrigin::signed(account(1)), UNSTAKE_AMOUNT, None),
			Error::<Runtime>::UnstakeAmountExceedStakedAmount
		);
	});
}

#[test]
fn unstake_should_fail_unstake_zero() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_noop!(
			EconomyModule::unstake(RuntimeOrigin::signed(account(1)), 0u128, None),
			Error::<Runtime>::UnstakeAmountIsZero
		);
	});
}

#[test]
fn unstake_should_fail_for_non_existing_estate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			EconomyModule::unstake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE, Some(8u32.into())),
			Error::<Runtime>::StakeEstateDoesNotExist
		);
	});
}

#[test]
fn unstake_should_fail_for_estate_the_account_has_not_staked_in() {
	ExtBuilder::default().build().execute_with(|| {
		let prepopulated_bond = Bond {
			staker: account(2),
			amount: STAKE_BALANCE,
		};

		EstateStakingInfo::<Runtime>::insert(&OWNED_ESTATE_ID, prepopulated_bond);

		assert_noop!(
			EconomyModule::unstake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE, Some(OWNED_ESTATE_ID)),
			Error::<Runtime>::NoFundsStakedAtEstate
		);
	});
}

#[test]
fn unstake_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_ok!(EconomyModule::unstake(
			RuntimeOrigin::signed(account(1)),
			UNSTAKE_AMOUNT,
			None
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::SelfStakingRemovedFromEconomy101(
				account(1),
				UNSTAKE_AMOUNT
			))
		);

		let total_staked_balance = STAKE_BALANCE - UNSTAKE_AMOUNT;

		assert_eq!(EconomyModule::get_staking_info(account(1)), total_staked_balance);
		assert_eq!(EconomyModule::total_stake(), total_staked_balance);
		let next_round: RoundIndex = CURRENT_ROUND.saturating_add(1);
		assert_eq!(
			EconomyModule::staking_exit_queue(account(1), next_round),
			Some(UNSTAKE_AMOUNT)
		);
	});
}

#[test]
fn unstake_should_work_for_estate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			Some(OWNED_ESTATE_ID)
		));

		assert_noop!(
			EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE, Some(OWNED_ESTATE_ID)),
			Error::<Runtime>::StakeAmountExceedMaximumAmount
		);

		assert_ok!(EconomyModule::unstake(
			RuntimeOrigin::signed(account(1)),
			UNSTAKE_AMOUNT,
			Some(OWNED_ESTATE_ID)
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::EstateStakingRemovedFromEconomy101(
				account(1),
				OWNED_ESTATE_ID,
				UNSTAKE_AMOUNT
			))
		);

		let total_staked_balance = STAKE_BALANCE - UNSTAKE_AMOUNT;
		assert_eq!(
			EconomyModule::get_estate_staking_info(OWNED_ESTATE_ID).unwrap().staker,
			account(1)
		);
		assert_eq!(
			EconomyModule::get_estate_staking_info(OWNED_ESTATE_ID).unwrap().amount,
			total_staked_balance
		);
		assert_eq!(EconomyModule::total_estate_stake(), total_staked_balance);

		let next_round: RoundIndex = CURRENT_ROUND.saturating_add(1);
		assert_eq!(
			EconomyModule::estate_staking_exit_queue((account(1), next_round, OWNED_ESTATE_ID)),
			Some(UNSTAKE_AMOUNT)
		);
	});
}

#[test]
fn withdraw_unstake_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_ok!(EconomyModule::unstake(
			RuntimeOrigin::signed(account(1)),
			UNSTAKE_AMOUNT,
			None
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::SelfStakingRemovedFromEconomy101(
				account(1),
				UNSTAKE_AMOUNT
			))
		);

		let total_staked_balance = STAKE_BALANCE - UNSTAKE_AMOUNT;

		assert_eq!(EconomyModule::get_staking_info(account(1)), total_staked_balance);
		assert_eq!(EconomyModule::total_stake(), total_staked_balance);
		let next_round: RoundIndex = CURRENT_ROUND.saturating_add(1);
		assert_eq!(
			EconomyModule::staking_exit_queue(account(1), next_round),
			Some(UNSTAKE_AMOUNT)
		);

		// Default round length is 20 blocks so moving 25 blocks will move to the next round
		run_to_block(25);
		assert_ok!(EconomyModule::withdraw_unreserved(
			RuntimeOrigin::signed(account(1)),
			next_round
		));
		// account(1) balance free_balance was 9000 and added 9010 after withdraw unreserved
		assert_eq!(Balances::free_balance(account(1)), FREE_BALANCE);
	});
}

#[test]
fn unstake_should_work_with_single_round() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_ok!(EconomyModule::unstake(
			RuntimeOrigin::signed(account(1)),
			UNSTAKE_AMOUNT,
			None
		));

		assert_ok!(EconomyModule::stake(RuntimeOrigin::signed(account(2)), 200, None));

		let alice_staked_balance = STAKE_BALANCE - UNSTAKE_AMOUNT;

		assert_eq!(EconomyModule::get_staking_info(account(1)), alice_staked_balance);

		let total_staked_balance = alice_staked_balance + 200;
		assert_eq!(EconomyModule::total_stake(), total_staked_balance);
	});
}

#[test]
fn unstake_should_fail_with_existing_queue() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			None
		));

		assert_ok!(EconomyModule::unstake(
			RuntimeOrigin::signed(account(1)),
			UNSTAKE_AMOUNT,
			None
		));

		assert_ok!(EconomyModule::stake(RuntimeOrigin::signed(account(2)), 200, None));

		let alice_staked_balance = STAKE_BALANCE - UNSTAKE_AMOUNT;

		assert_eq!(EconomyModule::get_staking_info(account(1)), alice_staked_balance);

		let total_staked_balance = alice_staked_balance + 200;
		assert_eq!(EconomyModule::total_stake(), total_staked_balance);

		assert_noop!(
			EconomyModule::unstake(RuntimeOrigin::signed(account(1)), UNSTAKE_AMOUNT, None),
			Error::<Runtime>::ExitQueueAlreadyScheduled
		);
	});
}

#[test]
fn unstake_new_estate_owner_should_fail_if_estate_does_not_exist() {
	ExtBuilder::default().build().execute_with(|| {
		//assert_ok!(EconomyModule::stake(RuntimeOrigin::signed(account(1)), STAKE_BALANCE,
		// Some(OWNED_ESTATE_ID)));
		assert_noop!(
			EconomyModule::unstake_new_estate_owner(RuntimeOrigin::signed(account(1)), 1000u64),
			Error::<Runtime>::StakeEstateDoesNotExist
		);
	});
}

#[test]
fn unstake_new_estate_owner_should_fail_if_not_estate_owner() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			Some(OWNED_ESTATE_ID)
		));
		assert_noop!(
			EconomyModule::unstake_new_estate_owner(RuntimeOrigin::signed(account(2)), OWNED_ESTATE_ID),
			Error::<Runtime>::StakerNotEstateOwner
		);
	});
}

#[test]
fn unstake_new_estate_owner_should_fail_if_no_previous_owner_has_staked_balance_left() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(EconomyModule::stake(
			RuntimeOrigin::signed(account(1)),
			STAKE_BALANCE,
			Some(OWNED_ESTATE_ID)
		));
		assert_noop!(
			EconomyModule::unstake_new_estate_owner(RuntimeOrigin::signed(account(1)), OWNED_ESTATE_ID),
			Error::<Runtime>::StakerNotPreviousOwner
		);
	});
}

#[test]
fn unstake_new_estate_owner_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let prepopulated_bond = Bond {
			staker: account(2),
			amount: STAKE_BALANCE,
		};

		EstateStakingInfo::<Runtime>::insert(&OWNED_ESTATE_ID, prepopulated_bond);
		assert_ok!(EconomyModule::unstake_new_estate_owner(
			RuntimeOrigin::signed(account(1)),
			OWNED_ESTATE_ID
		));
		assert_eq!(
			last_event(),
			RuntimeEvent::Economy(crate::Event::EstateStakingRemovedFromEconomy101(
				account(1),
				OWNED_ESTATE_ID,
				STAKE_BALANCE
			))
		);
		assert_eq!(EconomyModule::get_estate_staking_info(OWNED_ESTATE_ID).is_some(), false);
		assert_eq!(EconomyModule::total_estate_stake(), 0u128);
	});
}
