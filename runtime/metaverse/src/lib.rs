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

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate orml_benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelaychainBlockNumberProvider;
// pub use this so we can import it in the chain spec.
#[cfg(feature = "std")]
pub use fp_evm::GenesisAccount;
use fp_rpc::TransactionStatus;
// use metaverse::weights::WeightInfo;
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::frame_support::pallet_prelude::Get;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, EnsureOrigin, KeyOwnerProofSystem, Randomness, StorageInfo, WithdrawReasons},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		ConstantMultiplier, DispatchClass, DispatchInfo, IdentityFee, Weight, WeightToFeePolynomial,
	},
	PalletId, RuntimeDebug, StorageValue,
};
use frame_support::{BoundedVec, ConsensusEngineId};
// A few exports that help ease life for downstream crates.
use frame_support::traits::{
	Contains, EitherOfDiverse, EnsureOneOf, EqualPrivilegeOnly, Everything, FindAuthor, InstanceFilter, Nothing,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	Config, EnsureRoot, EnsureSigned, RawOrigin,
};
use orml_traits::parameter_type_with_key;
pub use pallet_balances::Call as BalancesCall;
use pallet_contracts::weights::WeightInfo;
use pallet_ethereum::PostLogContent;
use pallet_ethereum::{Call::transact, EthereumBlockHashMapping, Transaction as EthereumTransaction};
use pallet_evm::GasWeightMapping;
use pallet_evm::{
	Account as EVMAccount, EnsureAddressNever, EnsureAddressRoot, FeeCalculator, HashedAddressMapping, Runner,
	SubstrateBlockHashMapping,
};
use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use polkadot_primitives::MAX_POV_SIZE;
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::Get;
use sp_core::{
	crypto::{KeyTypeId, Public},
	sp_std::marker::PhantomData,
	ConstBool, OpaqueMetadata, H160, H256, U256,
};
use sp_runtime::traits::{BlockNumberProvider, DispatchInfoOf};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, Bounded, ConvertInto, Dispatchable,
		IdentifyAccount, NumberFor, OpaqueKeys, PostDispatchInfoOf, UniqueSaturatedInto, Verify, Zero,
	},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError},
	ApplyExtrinsicResult, FixedPointNumber, MultiSignature, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

//use pallet_evm::{EnsureAddressTruncated, HashedAddressMapping};
use asset_manager::ForeignAssetMapping;
pub use constants::{currency::*, time::*};
use core_primitives::{NftAssetData, NftClassData};
// External imports
use currencies::BasicCurrencyAdapter;
pub use estate::{MintingRateInfo, Range as MintingRange};
use evm_mapping::EvmAddressMapping;
use metaverse_runtime_common::{precompiles::MetaverseNetworkPrecompiles, CurrencyHooks};
use primitives::evm::{
	CurrencyIdType, Erc20Mapping, EvmAddress, H160_POSITION_CURRENCY_ID_TYPE, H160_POSITION_FUNGIBLE_TOKEN,
	H160_POSITION_MINING_RESOURCE, H160_POSITION_TOKEN, H160_POSITION_TOKEN_NFT, H160_POSITION_TOKEN_NFT_CLASS_ID_END,
};
use primitives::{Amount, Balance, BlockNumber, ClassId, FungibleTokenId, Moment, NftId, PoolId, RoundIndex, TokenId};

// primitives imports
use crate::opaque::SessionKeys;
// EVM imports
use crate::sp_api_hidden_includes_construct_runtime::hidden_include::traits::Hooks;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

mod benchmarking;
mod weights;

/// Constant values used within the runtime.
pub mod constants;

/// Base storage fee
pub const BASE_STORAGE_FEE: Balance = 10 * CENTS;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

type EventRecord =
	frame_system::EventRecord<<Runtime as frame_system::Config>::RuntimeEvent, <Runtime as frame_system::Config>::Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.

pub mod opaque {
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	use super::*;

	//	pub type Block = BlockP;
	//	/// Opaque block header type.
	//	pub type Header = HeaderP;
	//	/// Opaque block identifier type.
	//	pub type BlockId = BlockIdP;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning and what each of the following value means:
//   https://substrate.dev/docs/en/knowledgebase/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("metaverse-runtime"),
	impl_name: create_runtime_str!("metaverse-runtime"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 101,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 3 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(2), MAX_POV_SIZE as u64);

// Prints debug output of the `contracts` pallet to stdout if the node is
// started with `-lruntime::contracts=debug`.
pub const CONTRACTS_DEBUG_OUTPUT: bool = true;

parameter_types! {
//	pub const Version: RuntimeVersion = VERSION;
//	pub const BlockHashCount: BlockNumber = 2400;
//	/// We allow for 2 seconds of compute with a 3 second average block time.
//	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
//		::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
//	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
//		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
//
//
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();

	pub const SS58Prefix: u8 = 42;
}

// Filter call that we don't enable before governance launch
// Allow base system calls needed for block production and runtime upgrade
// Other calls will be disallowed
pub struct NormalCallFilter;

impl Contains<RuntimeCall> for NormalCallFilter {
	fn contains(c: &RuntimeCall) -> bool {
		let is_parachain_call = matches!(
			c,
			// Calls from Sudo
			RuntimeCall::Sudo(..)
			// Calls for runtime upgrade.
			| RuntimeCall::System(..)
			| RuntimeCall::Timestamp(..)
			// Enable session
			| RuntimeCall::Session(..)
		);

		if is_parachain_call {
			// Allow parachain system call
			return true;
		}

		let is_emergency_stopped = emergency::EmergencyStoppedFilter::<Runtime>::contains(c);

		if is_emergency_stopped {
			// Not allow stopped tx
			return false;
		}
		true
	}
}

/// Maintenance mode Call filter
pub struct MaintenanceFilter;

impl Contains<RuntimeCall> for MaintenanceFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::Auction(_) => false,
			RuntimeCall::Balances(_) => false,
			RuntimeCall::Currencies(_) => false,
			RuntimeCall::Crowdloan(_) => false,
			RuntimeCall::Continuum(_) => false,
			RuntimeCall::Economy(_) => false,
			RuntimeCall::Estate(_) => false,
			RuntimeCall::Mining(_) => false,
			RuntimeCall::Metaverse(_) => false,
			RuntimeCall::Nft(_) => false,
			RuntimeCall::Treasury(_) => false,
			RuntimeCall::Vesting(_) => false,
			_ => true,
		}
	}
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Emergency;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.

	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MetaverseNetworkTreasuryPalletId: PalletId = PalletId(*b"bit/trsy");
	pub const NftPalletId: PalletId = PalletId(*b"bit/bnft");
	pub const SwapPalletId: PalletId = PalletId(*b"bit/swap");
	pub const BitMiningTreasury: PalletId = PalletId(*b"bit/ming");
	pub const EconomyTreasury: PalletId = PalletId(*b"bit/econ");
	pub const LocalMetaverseFundPalletId: PalletId = PalletId(*b"bit/meta");
	pub const BridgeSovereignPalletId: PalletId = PalletId(*b"bit/brgd");
	pub const PoolAccountPalletId: PalletId = PalletId(*b"bit/pool");
	pub const RewardPayoutAccountPalletId: PalletId = PalletId(*b"bit/pout");
	pub const RewardHoldingAccountPalletId: PalletId = PalletId(*b"bit/hold");

	pub const MaxAuthorities: u32 = 50;
	pub const MaxSetIdSessionEntries: u64 = u64::MAX;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type MaxAuthorities = MaxAuthorities;
	type DisabledValidators = ();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
	type PalletsOrigin = OriginCaller;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = MILLICENTS;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate =
		TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier, MaximumMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 10;
}

// Council & Technical committee related pallets
type CouncilCollective = pallet_collective::Instance1;
type TechnicalCommitteeCollective = pallet_collective::Instance2;

// Council
pub type EnsureRootOrAllCouncilCollective = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
>;

type EnsureRootOrHalfCouncilCollective = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

type EnsureRootOrTwoThirdsCouncilCollective = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
>;

// Technical Committee

pub type EnsureRootOrAllTechnicalCommittee = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 1, 1>,
>;

type EnsureRootOrHalfTechnicalCommittee = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 1, 2>,
>;

type EnsureRootOrTwoThirdsTechnicalCommittee = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 2, 3>,
>;

impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
}

parameter_types! {
	pub const TechnicalCommitteeMotionDuration: BlockNumber = 5 * DAYS;
	pub const TechnicalCommitteeMaxProposals: u32 = 100;
	pub const TechnicalCouncilMaxMembers: u32 = 3;
	pub MaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

impl pallet_collective::Config<TechnicalCommitteeCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalCommitteeMotionDuration;
	type MaxProposals = TechnicalCommitteeMaxProposals;
	type MaxMembers = TechnicalCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
}

// Metaverse network related pallets

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: FungibleTokenId| -> Balance {
		Zero::zero()
	};
}

parameter_types! {
	pub TreasuryModuleAccount: AccountId = MetaverseNetworkTreasuryPalletId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = FungibleTokenId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = CurrencyHooks<Runtime, TreasuryModuleAccount>;
	type MaxLocks = MaxLocks;
	type ReserveIdentifier = [u8; 8];
	type MaxReserves = ();
	type DustRemovalWhitelist = Nothing;
}

parameter_types! {
	pub const GetNativeCurrencyId: FungibleTokenId = FungibleTokenId::NativeToken(0);
}

impl currencies::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiSocialCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = weights::module_currencies::WeightInfo<Runtime>;
}

parameter_types! {
	pub AssetMintingFee: Balance = 1 * DOLLARS;
	pub ClassMintingFee: Balance = 10 * DOLLARS;
	pub MaxBatchTransfer: u32 = 100;
	pub MaxBatchMinting: u32 = 1000;
	pub MaxNftMetadata: u32 = 1024;
	pub const StorageDepositFee: Balance = BASE_STORAGE_FEE;
}

impl nft::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type Treasury = MetaverseNetworkTreasuryPalletId;
	type WeightInfo = weights::module_nft::WeightInfo<Runtime>;
	type PalletId = NftPalletId;
	type AuctionHandler = Auction;
	type MaxBatchTransfer = MaxBatchTransfer;
	type MaxBatchMinting = MaxBatchMinting;
	type MaxMetadata = MaxNftMetadata;
	type MiningResourceId = MiningResourceCurrencyId;
	type AssetMintingFee = AssetMintingFee;
	type ClassMintingFee = ClassMintingFee;
	type StorageDepositFee = StorageDepositFee;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
}

parameter_types! {
	pub MaxClassMetadata: u32 = 1024;
	pub MaxTokenMetadata: u32 = 1024;
}

impl orml_nft::Config for Runtime {
	type ClassId = ClassId;
	type TokenId = NftId;
	type Currency = Balances;
	type ClassData = NftClassData<Balance>;
	type TokenData = NftAssetData<Balance>;
	type MaxClassMetadata = MaxClassMetadata;
	type MaxTokenMetadata = MaxTokenMetadata;
}

parameter_types! {
	pub MaxMetaverseMetadata: u32 = 1024;
	pub MinContribution: Balance = 50 * DOLLARS;
	pub MaxNumberOfStakerPerMetaverse: u32 = 512;
	pub MetaverseStorageFee: Balance = 2 * BASE_STORAGE_FEE;
}

impl metaverse::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MetaverseTreasury = LocalMetaverseFundPalletId;
	type NetworkTreasury = TreasuryModuleAccount;
	type Currency = Balances;
	type MaxMetaverseMetadata = MaxMetaverseMetadata;
	type MinContribution = MinContribution;
	type MetaverseCouncil = EnsureRootOrHalfCouncilCollective;
	type WeightInfo = weights::module_metaverse::WeightInfo<Runtime>;
	type MetaverseRegistrationDeposit = MinContribution;
	type MinStakingAmount = MinContribution;
	type MaxNumberOfStakersPerMetaverse = MaxNumberOfStakerPerMetaverse;
	type MultiCurrency = Currencies;
	type NFTHandler = Nft;
	type StorageDepositFee = MetaverseStorageFee;
}

parameter_types! {
	pub const MinimumLandPrice: Balance = 10 * DOLLARS;
	pub const LandTreasuryPalletId: PalletId = PalletId(*b"bit/land");
	pub const MinBlocksPerLandIssuanceRound: u32 = 20;
	pub const MinimumStake: Balance = 100 * DOLLARS;
	pub const RewardPaymentDelay: u32 = 2;
	pub const DefaultMaxBound: (i32,i32) = (-1000,1000);
	pub const NetworkFee: Balance = 10 * DOLLARS; // Network fee
	pub const MaxOffersPerEstate: u32 = 100;
	pub const MinLeasePricePerBlock: Balance = 1 * CENTS;
	pub const MaxLeasePeriod: u32 = 1000000;
	pub const LeaseOfferExpiryPeriod: u32 = 10000;
	pub const MaximumEstateStake: Balance = 1000 * DOLLARS;
	pub const EstateStorageFee: Balance =  BASE_STORAGE_FEE;
}

impl estate::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type LandTreasury = LandTreasuryPalletId;
	type MetaverseInfoSource = Metaverse;
	type Currency = Balances;
	type MinimumLandPrice = MinimumLandPrice;
	type CouncilOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	type AuctionHandler = Auction;
	type MinBlocksPerRound = MinBlocksPerLandIssuanceRound;
	type WeightInfo = weights::module_estate::WeightInfo<Runtime>;
	type MinimumStake = MinimumStake;
	type RewardPaymentDelay = RewardPaymentDelay;
	type NFTTokenizationSource = Nft;
	type DefaultMaxBound = DefaultMaxBound;
	type NetworkFee = NetworkFee;
	type MaxOffersPerEstate = MaxOffersPerEstate;
	type MinLeasePricePerBlock = MinLeasePricePerBlock;
	type MaxLeasePeriod = MaxLeasePeriod;
	type LeaseOfferExpiryPeriod = LeaseOfferExpiryPeriod;
	type BlockNumberToBalance = ConvertInto;
	type StorageDepositFee = EstateStorageFee;
}

parameter_types! {
	pub const AuctionTimeToClose: u32 = 100; // Default 100800 Blocks
	pub const ContinuumSessionDuration: BlockNumber = 100; // Default 43200 Blocks
	pub const SpotAuctionChillingDuration: BlockNumber = 100; // Default 43200 Blocks
	pub const MinimumAuctionDuration: BlockNumber = 30; // Minimum duration is 300 blocks
	pub const MaxFinality: u32 = 200; // Maximum finalize auctions per block
	pub const MaxBundleItem: u32 = 100; // Maximum finalize auctions per block
	pub const NetworkFeeReserve: Balance = 1 * DOLLARS; // Network fee reserved when item is listed for auction
	pub const NetworkFeeCommission: Perbill = Perbill::from_percent(1); // Network fee collected after an auction is over
	pub const OfferDuration: BlockNumber = 100800; // Default 100800 Blocks
	pub const MinimumListingPrice: Balance = DOLLARS;
	pub const AntiSnipeDuration: BlockNumber = 50; // Minimum anti snipe duration is 50 blocks
	pub const AuctionStorageFee: Balance = 3 * BASE_STORAGE_FEE;
}

impl auction::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AuctionTimeToClose = AuctionTimeToClose;
	type Handler = Auction;
	type Currency = Balances;
	type ContinuumHandler = Continuum;
	type FungibleTokenCurrency = Tokens;
	type MetaverseInfoSource = Metaverse;
	type MinimumAuctionDuration = MinimumAuctionDuration;
	type EstateHandler = Estate;
	type MaxFinality = MaxFinality;
	type NFTHandler = Nft;
	type MaxBundleItem = MaxBundleItem;
	type NetworkFeeReserve = NetworkFeeReserve;
	type NetworkFeeCommission = NetworkFeeCommission;
	type WeightInfo = weights::module_auction::WeightInfo<Runtime>;
	type OfferDuration = OfferDuration;
	type MinimumListingPrice = MinimumListingPrice;
	type AntiSnipeDuration = AntiSnipeDuration;
	type StorageDepositFee = AuctionStorageFee;
}

parameter_types! {
	pub const ContinuumStorageDeposit: Balance = BASE_STORAGE_FEE;
}
impl continuum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SessionDuration = ContinuumSessionDuration;
	type SpotAuctionChillingDuration = SpotAuctionChillingDuration;
	type EmergencyOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 3>;
	type AuctionHandler = Auction;
	type AuctionDuration = SpotAuctionChillingDuration;
	type ContinuumTreasury = MetaverseNetworkTreasuryPalletId;
	type Currency = Balances;
	type MetaverseInfoSource = Metaverse;
	type StorageDepositFee = ContinuumStorageDeposit;
	type WeightInfo = weights::module_continuum::WeightInfo<Runtime>;
}

pub struct EnsureRootOrMetaverseTreasury;

impl EnsureOrigin<RuntimeOrigin> for EnsureRootOrMetaverseTreasury {
	type Success = AccountId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Into::<Result<RawOrigin<AccountId>, RuntimeOrigin>>::into(o).and_then(|o| match o {
			RawOrigin::Root => Ok(MetaverseNetworkTreasuryPalletId::get().into_account_truncating()),
			RawOrigin::Signed(caller) => {
				if caller == MetaverseNetworkTreasuryPalletId::get().into_account_truncating() {
					Ok(caller)
				} else {
					Err(RuntimeOrigin::from(Some(caller)))
				}
			}
			r => Err(RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		let zero_account_id = AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
			.expect("infinite length input; no invalid inputs for type; qed");
		Ok(RuntimeOrigin::from(RawOrigin::Signed(zero_account_id)))
	}
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 10;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	const MAX_VESTING_SCHEDULES: u32 = 100;
}

parameter_types! {
	//Mining Resource Currency Id
	pub const MiningResourceCurrencyId: FungibleTokenId = FungibleTokenId::MiningResource(0);
	pub const TreasuryStakingReward: Perbill = Perbill::from_percent(1);
	pub MiningStorageDeposit: Balance = BASE_STORAGE_FEE;
}

impl mining::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MiningCurrency = Currencies;
	type BitMiningTreasury = BitMiningTreasury;
	type BitMiningResourceId = MiningResourceCurrencyId;
	type EstateHandler = Estate;
	type AdminOrigin = EnsureRootOrMetaverseTreasury;
	type MetaverseStakingHandler = Metaverse;
	type TreasuryStakingReward = TreasuryStakingReward;
	type NetworkTreasuryAccount = TreasuryModuleAccount;
	type StorageDepositFee = MiningStorageDeposit;
	type Currency = Balances;
	type WeightInfo = weights::module_mining::WeightInfo<Runtime>;
}

parameter_types! {
	pub const Period: u32 = DAYS;
	pub const Offset: u32 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"bcPotStk");
	pub const MaxCandidates: u32 = 10;
	pub const MinCandidates: u32 = 5;
	pub const MaxInvulnerables: u32 = 100;
}

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MinCandidates = MinCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

parameter_types! {
	pub PreimageBaseDeposit: Balance = deposit(2, 64);
	pub PreimageByteDeposit: Balance = 1 * CENTS;
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

pub struct FindAuthorTruncated<F>(PhantomData<F>);

impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		//		if let Some(author_index) = F::find_author(digests) {
		//			let authority_id = Aura::authorities()[author_index as usize].clone();
		//			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
		//		}
		None
	}
}

parameter_types! {
	pub const OneBlock: BlockNumber = 1;
	pub const MinimumProposalDeposit: Balance = 50 * DOLLARS;
	pub const DefaultPreimageByteDeposit: Balance = 1 * DOLLARS;
	pub const DefaultVotingPeriod: u32 = 100;
	pub const DefaultLocalVoteLockingPeriod: u32 = 28;
	pub const DefaultEnactmentPeriod: u32 = 10;
	pub const DefaultProposalLaunchPeriod: u32 = 15;
	pub const DefaultMaxProposalsPerMetaverse: u8 = 20;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 1 * DAYS;
	pub const VotingPeriod: BlockNumber = 3 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 15 * MINUTES;
	pub const InstantAllowed: bool = true;
	pub const MinimumDeposit: Balance = 1000 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 6 * HOURS;
	pub const CooloffPeriod: BlockNumber = 1 * HOURS;
	pub const MaxVotes: u32 = 50;
	pub const MaxProposals: u32 = 50;
	pub const MaxBlacklisted: u32 = 100;
	pub const MaxDeposits: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod;
	type MinimumDeposit = MinimumDeposit;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EnsureRootOrHalfCouncilCollective;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EnsureRootOrHalfCouncilCollective;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = EnsureRootOrAllCouncilCollective;
	//type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type InstantOrigin = EnsureRootOrAllTechnicalCommittee;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = EnsureRootOrTwoThirdsCouncilCollective;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureRootOrAllTechnicalCommittee;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = ();
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = MaxDeposits;
	type MaxBlacklisted = MaxBlacklisted;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub ProxyDepositFactor: Balance = deposit(0, 33);
	pub AnnouncementDepositBase: Balance = deposit(1, 8);
	pub AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum ProxyType {
	Any,
	CancelProxy,
	Governance,
	Auction,
	Economy,
	Nft,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			_ if matches!(c, RuntimeCall::Utility(..)) => true,
			ProxyType::Any => true,
			ProxyType::CancelProxy => matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..) | RuntimeCall::Council(..) | RuntimeCall::TechnicalCommittee(..)
			),
			ProxyType::Auction => matches!(
				c,
				RuntimeCall::Auction(auction::Call::bid { .. }) | RuntimeCall::Auction(auction::Call::buy_now { .. })
			),
			ProxyType::Economy => matches!(
				c,
				RuntimeCall::Economy(economy::Call::stake { .. }) | RuntimeCall::Economy(economy::Call::unstake { .. })
			),
			ProxyType::Nft => matches!(
				c,
				RuntimeCall::Nft(nft::Call::transfer { .. }) | RuntimeCall::Nft(nft::Call::transfer_batch { .. })
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = ();
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ProposalType {
	Any,
	JustMetaverse,
}

impl Default for ProposalType {
	fn default() -> Self {
		Self::JustMetaverse
	}
}

impl InstanceFilter<RuntimeCall> for ProposalType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProposalType::Any => true,
			ProposalType::JustMetaverse => matches!(c, RuntimeCall::Metaverse(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		self == &ProposalType::Any || self == o
	}
}

parameter_types! {
	pub GovernanceStorageFee: Balance = BASE_STORAGE_FEE;
}

impl governance::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DefaultPreimageByteDeposit = DefaultPreimageByteDeposit;
	type MinimumProposalDeposit = MinimumProposalDeposit;
	type DefaultProposalLaunchPeriod = DefaultProposalLaunchPeriod;
	type DefaultVotingPeriod = DefaultVotingPeriod;
	type DefaultEnactmentPeriod = DefaultEnactmentPeriod;
	type DefaultLocalVoteLockingPeriod = DefaultLocalVoteLockingPeriod;
	type DefaultMaxProposalsPerMetaverse = DefaultMaxProposalsPerMetaverse;
	type OneBlock = OneBlock;
	type Currency = Balances;
	type Slash = ();
	type MetaverseInfo = Metaverse;
	type PalletsOrigin = OriginCaller;
	type Proposal = RuntimeCall;
	type Scheduler = Scheduler;
	type MetaverseLandInfo = Estate;
	type MetaverseCouncil = EnsureRootOrMetaverseTreasury;
	type ProposalType = ProposalType;
	type NetworkTreasury = TreasuryModuleAccount;
	type StorageDepositFee = GovernanceStorageFee;
}

impl crowdloan::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VestingSchedule = Vesting;
	type BlockNumberToBalance = ConvertInto;
	type WeightInfo = ();
}
parameter_types! {
	pub const MiningCurrencyId: FungibleTokenId = FungibleTokenId::MiningResource(0);
	pub const PowerAmountPerBlock: u32 = 100;
}

impl economy::Config for Runtime {
	type Currency = Balances;
	type EconomyTreasury = EconomyTreasury;
	type RuntimeEvent = RuntimeEvent;
	type FungibleTokenCurrency = Currencies;
	type MinimumStake = MinimumStake;
	type MiningCurrencyId = MiningCurrencyId;
	type NFTHandler = Nft;
	type EstateHandler = Estate;
	type RoundHandler = Mining;
	type PowerAmountPerBlock = PowerAmountPerBlock;
	type WeightInfo = weights::module_economy::WeightInfo<Runtime>;
	type MaximumEstateStake = MaximumEstateStake;
}

impl emergency::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type EmergencyOrigin = EnsureRootOrHalfCouncilCollective;
	type NormalCallFilter = NormalCallFilter;
	type MaintenanceCallFilter = MaintenanceFilter;
	type WeightInfo = weights::module_emergency::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumCount: u32 = 5;
	pub const ExpiresIn: Moment = 1000 * 60 * 60 * 24; // 24 hours
	pub RootOperatorAccountId: AccountId = AccountId::from([0xffu8; 32]);
	pub const MaxHasDispatchedSize: u32 = 20;
	pub const OracleMaxMembers: u32 = 50;
	pub const MaxFeedValues: u32 = 10; // max 10 values allowd to feed in one call.
}

pub type OracleMembershipInstance = pallet_membership::Instance1;

impl pallet_membership::Config<OracleMembershipInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrHalfCouncilCollective;
	type RemoveOrigin = EnsureRootOrHalfCouncilCollective;
	type SwapOrigin = EnsureRootOrHalfCouncilCollective;
	type ResetOrigin = EnsureRootOrHalfCouncilCollective;
	type PrimeOrigin = EnsureRootOrHalfCouncilCollective;
	type MembershipInitialized = ();
	type MembershipChanged = RewardOracle;
	type MaxMembers = OracleMaxMembers;
	type WeightInfo = ();
}

type MiningRewardDataProvider = orml_oracle::Instance1;

impl orml_oracle::Config<MiningRewardDataProvider> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn, MiningRewardDataProvider>;
	type Time = Timestamp;
	type OracleKey = RoundIndex;
	type OracleValue = BoundedVec<u8, MaxMetaverseMetadata>;
	type RootOperatorAccountId = RootOperatorAccountId;
	type Members = OracleMembership;
	type MaxHasDispatchedSize = MaxHasDispatchedSize;
	type MaxFeedValues = MaxFeedValues;
	type WeightInfo = ();
}

parameter_types! {
	// Tells `pallet_base_fee` whether to calculate a new BaseFee `on_finalize` or not.
	pub DefaultBaseFeePerGas: U256 = (10 * CENTS).into();
	// Not using dynamic fee calculation
	pub DefaultElasticity: Permill = Permill::zero();
}

pub struct BaseFeeThreshold;

impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND.saturating_div(GAS_PER_SECOND);

parameter_types! {
	/// EVM gas limit
	pub BlockGasLimit: U256 = U256::from(
		NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS
	);
	pub PrecompilesValue: MetaverseNetworkPrecompiles<Runtime> = MetaverseNetworkPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_ref_time(WEIGHT_PER_GAS);
	pub const GasLimitPovSizeRatio: u64 = 4;
}

impl pallet_evm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;

	type BlockGasLimit = BlockGasLimit;
	// Ethereum-compatible chain_id:
	// * Metaverse Network: 2042
	type ChainId = EvmChainId;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Runner = pallet_evm::runner::stack::Runner<Self>;

	type CallOrigin = EnsureAddressRoot<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;

	type FeeCalculator = ();
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type OnChargeTransaction = ();
	type FindAuthor = FindAuthorTruncated<Aura>;
	type PrecompilesType = MetaverseNetworkPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type WeightPerGas = WeightPerGas;
	type Timestamp = Timestamp;
	type OnCreate = ();
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
	type PostLogContent = PostBlockAndTxnHashes;
	// Maximum length (in bytes) of revert message to include in Executed event
	type ExtraDataLength = ConstU32<30>;
}

pub struct RPCCallFilter;

impl Contains<RuntimeCall> for RPCCallFilter {
	fn contains(c: &RuntimeCall) -> bool {
		matches!(c, RuntimeCall::Currencies(..))
	}
}

/// Evm address mapping
impl Erc20Mapping for Runtime {
	fn encode_evm_address(t: FungibleTokenId) -> Option<EvmAddress> {
		EvmAddress::try_from(t).ok()
	}

	fn encode_nft_evm_address(t: (ClassId, TokenId)) -> Option<EvmAddress> {
		let mut address = [2u8; 20];
		let mut asset_bytes: Vec<u8> = t.0.to_be_bytes().to_vec();
		asset_bytes.append(&mut t.1.to_be_bytes().to_vec());

		for byte_index in 0..asset_bytes.len() {
			address[byte_index + H160_POSITION_TOKEN_NFT.start] = asset_bytes.as_slice()[byte_index];
		}

		Some(EvmAddress::from_slice(&address))
	}

	fn decode_evm_address(addr: EvmAddress) -> Option<FungibleTokenId> {
		let address = addr.as_bytes();
		let currency_id = match CurrencyIdType::try_from(address[H160_POSITION_CURRENCY_ID_TYPE]).ok()? {
			CurrencyIdType::NativeToken => address[H160_POSITION_TOKEN]
				.try_into()
				.map(FungibleTokenId::NativeToken)
				.ok(),
			CurrencyIdType::MiningResource => address[H160_POSITION_TOKEN]
				.try_into()
				.map(FungibleTokenId::MiningResource)
				.ok(),
			CurrencyIdType::FungibleToken => address[H160_POSITION_TOKEN]
				.try_into()
				.map(FungibleTokenId::FungibleToken)
				.ok(),
		};

		// Encode again to ensure encoded address is matched
		Self::encode_evm_address(currency_id?).and_then(|encoded| if encoded == addr { currency_id } else { None })
	}

	fn decode_nft_evm_address(addr: EvmAddress) -> Option<(ClassId, TokenId)> {
		let address = addr.as_bytes();

		let mut class_id_bytes = [2u8; 4];
		let mut token_id_bytes = [2u8; 8];
		for byte_index in H160_POSITION_TOKEN_NFT {
			if byte_index < H160_POSITION_TOKEN_NFT_CLASS_ID_END {
				class_id_bytes[byte_index - H160_POSITION_TOKEN_NFT.start] = address[byte_index];
			} else {
				token_id_bytes[byte_index - H160_POSITION_TOKEN_NFT_CLASS_ID_END] = address[byte_index];
			}
		}

		let class_id = u32::from_be_bytes(class_id_bytes);
		let token_id = u64::from_be_bytes(token_id_bytes);

		// Encode again to ensure encoded address is matched
		Self::encode_nft_evm_address((class_id, token_id)).and_then(|encoded| {
			if encoded == addr {
				Some((class_id, token_id))
			} else {
				None
			}
		})
	}
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
	// Fallback value if storage deposit limit not set by the user
	pub const DefaultDepositLimit: Balance = deposit(16, 16 * 1024);
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = RPCCallFilter;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type DefaultDepositLimit = DefaultDepositLimit;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 5];
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<1024>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
}

// Treasury and Bounty
parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 50 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0); // No burn
	pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = MetaverseNetworkTreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EnsureRootOrHalfCouncilCollective;
	type RejectOrigin = EnsureRootOrHalfCouncilCollective;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = ();
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	type ProposalBondMaximum = ProposalBondMaximum;
}

parameter_types! {
	pub const CampaignDeposit: Balance = 1 * DOLLARS;
	pub const MinimumRewardPool: Balance = 100 * DOLLARS;
	pub const MinimumCampaignCoolingOffPeriod: BlockNumber = 2; //  4 * 30 * 7200 Around 4 months in blocktime
	pub const MinimumCampaignDuration: BlockNumber = 1; // 7 * 7200 Around a week in blocktime
	pub const MaxLeafNodes: u64 = 30;
	pub const MaxSetRewardsListLength: u64 = 500;
	pub const RewardStorageFee: Balance = BASE_STORAGE_FEE;
}

impl reward::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FungibleTokenCurrency = Currencies;
	type PalletId = MetaverseNetworkTreasuryPalletId;
	type MiningCurrencyId = MiningResourceCurrencyId;
	type MinimumRewardPool = MinimumRewardPool;
	type CampaignDeposit = CampaignDeposit;
	type MinimumCampaignDuration = MinimumCampaignDuration;
	type MinimumCampaignCoolingOffPeriod = MinimumCampaignCoolingOffPeriod;
	type MaxSetRewardsListLength = MaxSetRewardsListLength;
	type AdminOrigin = EnsureRootOrMetaverseTreasury;
	type NFTHandler = Nft;
	type MaxLeafNodes = MaxLeafNodes;
	type StorageDepositFee = StorageDepositFee;
	type WeightInfo = weights::module_reward::WeightInfo<Runtime>;
}

impl asset_manager::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureRootOrHalfCouncilCollective;
}

pub type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Runtime {
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub EvmMappingStorageFee: Balance = 2 * BASE_STORAGE_FEE; // Each extrinsic in the pallet has exactly 2 storage inserts

}

impl evm_mapping::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type AddressMapping = EvmAddressMapping<Runtime>;
	type ChainId = EvmChainId;
	type TransferAll = OrmlCurrencies;
	type NetworkTreasuryAccount = TreasuryModuleAccount;
	type StorageDepositFee = EvmMappingStorageFee;
	type WeightInfo = weights::module_evm_mapping::WeightInfo<Runtime>;
}

impl modules_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeOrigin = EnsureRootOrTwoThirdsCouncilCollective;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type NFTHandler = Nft;
	type NativeCurrencyId = GetNativeCurrencyId;
	type PalletId = BridgeSovereignPalletId;
}

impl pallet_evm_chain_id::Config for Runtime {}

impl orml_rewards::Config for Runtime {
	type Share = Balance;
	type Balance = Balance;
	type PoolId = PoolId;
	type CurrencyId = FungibleTokenId;
	type Handler = Spp;
}

parameter_types! {
	pub const MaximumQueue: u32 = 50;
	pub const MockRelayBlockNumberProvider: BlockNumber = 0;
}

impl BlockNumberProvider for MockRelayBlockNumberProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		Self::get()
	}
}

impl spp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type WeightInfo = weights::module_spp::WeightInfo<Runtime>;
	type MinimumStake = MinimumStake;
	type NetworkFee = NetworkFee;
	type StorageDepositFee = StorageDepositFee;
	type RelayChainBlockNumber = MockRelayBlockNumberProvider;
	type PoolAccount = PoolAccountPalletId;
	type RewardPayoutAccount = RewardPayoutAccountPalletId;
	type RewardHoldingAccount = RewardHoldingAccountPalletId;
	type MaximumQueue = MaximumQueue;
	type CurrencyIdConversion = ForeignAssetMapping<Runtime>;
	type GovernanceOrigin = EnsureRootOrTwoThirdsCouncilCollective;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Core
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Aura: pallet_aura::{Pallet, Config<T>},
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
		Utility: pallet_utility::{Pallet, Call, Event},

		// Governance
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},

		// Token & Related
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: currencies::{ Pallet, Storage, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
		OrmlCurrencies: orml_currencies::{Pallet, Call},

		// Metaverse & Related
		OrmlNFT: orml_nft::{Pallet, Storage},
		Nft: nft::{Pallet, Call, Storage, Event<T>},
		Auction: auction::{Pallet, Call ,Storage, Event<T>},
		Metaverse: metaverse::{Pallet, Call, Storage, Event<T>},
		Continuum: continuum::{Pallet, Call, Storage, Event<T>},
		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>},
		Mining: mining::{Pallet, Call, Storage ,Event<T>},
		Reward: reward::{Pallet, Call, Storage ,Event<T>},
		Estate: estate::{Pallet, Call, Storage, Event<T>, Config},
		Economy: economy::{Pallet, Call, Storage, Event<T>},
		Emergency: emergency::{Pallet, Call, Storage, Event<T>},
		RewardOracle: orml_oracle::<Instance1>::{Pallet, Storage, Call, Event<T>},
		OracleMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Governance
		Governance: governance::{Pallet, Call ,Storage, Event<T>},
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},

		// External consensus support
		CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		// Crowdloan
		Crowdloan: crowdloan::{Pallet, Call, Storage, Event<T>},

		EVM: pallet_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin},
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event},
		EvmMapping: evm_mapping::{Pallet, Call, Storage, Event<T>},
		EvmChainId: pallet_evm_chain_id::{Pallet, Storage, Config},

		// ink! Smart Contracts.
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},

		// Technical committee
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage ,Origin<T>, Event<T>},
		Treasury: pallet_treasury::{Pallet, Storage, Config, Event<T>, Call},

		// Asset manager
		AssetManager: asset_manager::{Pallet, Call, Storage, Event<T>},

		// Proxy
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},

		// Bridge
		BridgeSupport: modules_bridge::{Pallet, Call, Storage, Event<T>},

		// Spp
		Spp: spp::{Pallet, Call, Storage, Event<T>},
		Rewards: orml_rewards::{Pallet, Storage}
	}
);

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(pallet_ethereum::Call::<Runtime>::transact { transaction }.into())
	}
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> opaque::UncheckedExtrinsic {
		let extrinsic =
			UncheckedExtrinsic::new_unsigned(pallet_ethereum::Call::<Runtime>::transact { transaction }.into());
		let encoded = extrinsic.encode();
		opaque::UncheckedExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[auction, benchmarking::auction]
		[continuum, benchmarking::continuum]
		[economy, benchmarking::economy]
		[estate, benchmarking::estate]
		[metaverse, benchmarking::metaverse]
		[reward, benchmarking::reward]
	);
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		origin: &H160,
		dispatch_info: &DispatchInfo,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(origin, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
				RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info)),
			)),
			_ => None,
		}
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			EvmChainId::get()
		}

		fn account_basic(address: H160) -> pallet_evm::Account {
			let (account, _) = EVM::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			pallet_evm::AccountCodes::<Runtime>::get(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;

			// Reused approach from Moonbeam since Frontier implementation doesn't support this
			let mut estimated_transaction_len = data.len() +
				// to: 20
				// from: 20
				// value: 32
				// gas_limit: 32
				// nonce: 32
				// 1 byte transaction action variant
				// chain id 8 bytes
				// 65 bytes signature
				210;
			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
			let without_base_extrinsic_weight = true;
			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				Vec::new(),
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				config
					.as_ref()
					.unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
			)
			.map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;


			// Reused approach from Moonbeam since Frontier implementation doesn't support this
			let mut estimated_transaction_len = data.len() +
				// to: 20
				// from: 20
				// value: 32
				// gas_limit: 32
				// nonce: 32
				// 1 byte transaction action variant
				// chain id 8 bytes
				// 65 bytes signature
				210;
			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
			let without_base_extrinsic_weight = true;
			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};


			#[allow(clippy::or_fun_call)] // suggestion not helpful here
			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				Vec::new(),
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				config
					.as_ref()
					.unwrap_or(<Runtime as pallet_evm::Config>::config()),
				)
				.map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			pallet_ethereum::CurrentBlock::<Runtime>::get()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			pallet_ethereum::CurrentReceipts::<Runtime>::get()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentReceipts::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact{transaction}) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(Permill::zero())
		}
		fn gas_limit_multiplier_support() {}

		fn pending_block(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> (Option<pallet_ethereum::Block>, Option<Vec<fp_rpc::TransactionStatus>>) {
			for ext in xts.into_iter() {
				let _ = Executive::apply_extrinsic(ext);
			}

			Ethereum::on_finalize(System::block_number() + 1);

			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(
			transaction: pallet_ethereum::Transaction
		) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts_primitives::ContractExecResult<Balance, EventRecord> {
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				storage_deposit_limit,
				input_data,
				pallet_contracts::DebugInfo::UnsafeDebug,
				pallet_contracts::CollectEvents::UnsafeCollect,
				pallet_contracts::Determinism::Enforced,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts_primitives::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_instantiate(
				origin,
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				salt,
				pallet_contracts::DebugInfo::UnsafeDebug,
				pallet_contracts::CollectEvents::UnsafeCollect,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: pallet_contracts::Determinism,
		) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts_primitives::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use orml_benchmarking::list_benchmark as orml_list_benchmark;
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use nft::benchmarking::Pallet as NftBench;
			use crowdloan::benchmarking::CrowdloanModule as CrowdloanBench;
			use mining::benchmarking::MiningModule as MiningBench;
			use currencies::benchmarking::CurrencyModule as CurrenciesBench;
			use emergency::benchmarking::EmergencyModule as EmergencyBench;
			use evm_mapping::benchmarking::EvmMappingModule as EvmMappingBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
			list_benchmark!(list, extra, pallet_balances, Balances);
			list_benchmark!(list, extra, pallet_timestamp, Timestamp);
			list_benchmark!(list, extra, nft, NftBench::<Runtime>);
			list_benchmark!(list, extra, crowdloan, CrowdloanBench::<Runtime>);
			list_benchmark!(list, extra, mining, MiningBench::<Runtime>);
			list_benchmark!(list, extra, pallet_utility, Utility);
			list_benchmark!(list, extra, currencies, CurrenciesBench::<Runtime>);
			list_benchmark!(list, extra, emergency, EmergencyBench::<Runtime>);
			list_benchmark!(list, extra, evm_mapping, EvmMappingBench::<Runtime>);
			orml_list_benchmark!(list, extra, auction, benchmarking::auction);
			orml_list_benchmark!(list, extra, continuum, benchmarking::continuum);
			orml_list_benchmark!(list, extra, economy, benchmarking::economy);
			orml_list_benchmark!(list, extra, estate, benchmarking::estate);
			orml_list_benchmark!(list, extra, metaverse, benchmarking::metaverse);
			orml_list_benchmark!(list, extra, reward, benchmarking::reward);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			use orml_benchmarking::add_benchmark as orml_add_benchmark;

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			use nft::benchmarking::Pallet as NftBench;
			use mining::benchmarking::MiningModule as MiningBench;
			use crowdloan::benchmarking::CrowdloanModule as CrowdloanBench;
			use currencies::benchmarking::CurrencyModule as CurrenciesBench;
			use emergency::benchmarking::EmergencyModule as EmergencyBench;
			use evm_mapping::benchmarking::EvmMappingModule as EvmMappingBench;

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);
			add_benchmark!(params, batches, nft, NftBench::<Runtime>);
			add_benchmark!(params, batches, crowdloan, CrowdloanBench::<Runtime>);
			add_benchmark!(params, batches, mining, MiningBench::<Runtime>);
			add_benchmark!(params, batches, pallet_utility, Utility);
			add_benchmark!(params, batches, currencies, CurrenciesBench::<Runtime>);
			add_benchmark!(params, batches, emergency, EmergencyBench::<Runtime>);
			add_benchmark!(params, batches, evm_mapping, EvmMappingBench::<Runtime>);
			orml_add_benchmark!(params, batches, auction, benchmarking::auction);
			orml_add_benchmark!(params, batches, continuum, benchmarking::continuum);
			orml_add_benchmark!(params, batches, economy, benchmarking::economy);
			orml_add_benchmark!(params, batches, estate, benchmarking::estate);
			orml_add_benchmark!(params, batches, metaverse, benchmarking::metaverse);
			orml_add_benchmark!(params, batches, reward, benchmarking::reward);
			Ok(batches)
		}
	}
}
