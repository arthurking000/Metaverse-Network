#![cfg(test)]

use frame_support::traits::{Contains, InstanceFilter, Nothing};
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, PalletId};
use frame_system::Call as SystemCall;
use frame_system::EnsureSignedBy;
use orml_traits::parameter_type_with_key;
use sp_core::crypto::AccountId32;
use sp_core::{ConstU128, H256};
use sp_runtime::traits::{BlakeTwo256, IdentifyAccount, Verify};
use sp_runtime::{testing::Header, traits::IdentityLookup, MultiSignature, Perbill};

use auction_manager::*;
use core_primitives::NftAssetData;
use primitives::{Amount, AuctionId, FungibleTokenId, ItemId};

use crate as reward;

use super::*;

pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Hash = H256;
type Signature = MultiSignature;
type AccountPublic = <Signature as Verify>::Signer;

pub const ALICE: AccountId = AccountId32::new([1; 32]);
pub const BOB: AccountId = AccountId32::new([2; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3; 32]);
pub const DONNA: AccountId = AccountId32::new([4; 32]);
pub const EVA: AccountId = AccountId32::new([5; 32]);

pub const COLLECTION_ID: u64 = 0;
pub const CLASS_ID: u32 = 0;

// Configure a mock runtime to test the pallet.
ord_parameter_types! {
	pub const One: AccountId = ALICE;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const EconomyPalletId: PalletId = PalletId(*b"bit/fund");
	pub const MiningTreasuryPalletId: PalletId = PalletId(*b"bit/fund");
	pub const MaxTokenMetadata: u32 = 1024;
	pub const MinimumStake: Balance = 100;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = frame_support::traits::ConstU32<0>;
	type MaxFreezes = frame_support::traits::ConstU32<0>;
}

parameter_types! {
	pub const CampaignTreasuryPalletId: PalletId = PalletId(*b"bit/fund");
	pub const CampaignDeposit: Balance = 1;
	pub const MinimumRewardPool: Balance = 1;
	pub const MinimumCampaignCoolingOffPeriod: BlockNumber = 10;
	pub const MinimumCampaignDuration: BlockNumber = 5;
	pub const MaxLeafNodes: u64 = 30;
	pub const MaxSetRewardsListLength: u64 = 2;
	pub StorageDepositFee: Balance = 1;
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FungibleTokenCurrency = Currencies;
	type MinimumRewardPool = MinimumRewardPool;
	type CampaignDeposit = CampaignDeposit;
	type PalletId = CampaignTreasuryPalletId;
	type MiningCurrencyId = MiningCurrencyId;
	type MinimumCampaignDuration = MinimumCampaignDuration;
	type MinimumCampaignCoolingOffPeriod = MinimumCampaignCoolingOffPeriod;
	type MaxSetRewardsListLength = MaxSetRewardsListLength;
	type AdminOrigin = EnsureSignedBy<One, AccountId>;
	type NFTHandler = NFTModule;
	type MaxLeafNodes = MaxLeafNodes;
	type WeightInfo = ();
	type StorageDepositFee = StorageDepositFee;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: FungibleTokenId| -> Balance {
		Default::default()
	};
}

parameter_types! {
	pub TreasuryModuleAccount: AccountId = EconomyPalletId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = FungibleTokenId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ();
	type DustRemovalWhitelist = Nothing;
	type ReserveIdentifier = [u8; 8];
	type MaxReserves = ();
}

pub type AdaptedBasicCurrency = currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

parameter_types! {
	pub const NativeCurrencyId: FungibleTokenId = FungibleTokenId::NativeToken(0);
	pub const MiningCurrencyId: FungibleTokenId = FungibleTokenId::MiningResource(0);
}

impl currencies::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiSocialCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = NativeCurrencyId;
	type WeightInfo = ();
}

pub struct MockAuctionManager;

impl Auction<AccountId, BlockNumber> for MockAuctionManager {
	type Balance = Balance;

	fn auction_info(_id: u64) -> Option<AuctionInfo<AccountId, Self::Balance, u64>> {
		None
	}

	fn auction_item(_id: AuctionId) -> Option<AuctionItem<AccountId, BlockNumber, Self::Balance>> {
		None
	}

	fn update_auction(_id: u64, _info: AuctionInfo<AccountId, Self::Balance, u64>) -> DispatchResult {
		Ok(())
	}

	fn update_auction_item(_id: AuctionId, _item_id: ItemId<Self::Balance>) -> DispatchResult {
		Ok(())
	}

	fn new_auction(
		_recipient: AccountId,
		_initial_amount: Self::Balance,
		_start: u64,
		_end: Option<u64>,
	) -> Result<u64, DispatchError> {
		Ok(1)
	}

	fn create_auction(
		_auction_type: AuctionType,
		_item_id: ItemId<Balance>,
		_end: Option<u64>,
		_recipient: AccountId,
		_initial_amount: Self::Balance,
		_start: u64,
		_listing_level: ListingLevel<AccountId>,
		_listing_fee: Perbill,
		_currency_id: FungibleTokenId,
	) -> Result<u64, DispatchError> {
		Ok(1)
	}

	fn remove_auction(_id: u64, _item_id: ItemId<Balance>) {}

	fn auction_bid_handler(_from: AccountId, _id: AuctionId, _value: Self::Balance) -> DispatchResult {
		Ok(())
	}

	fn buy_now_handler(_from: AccountId, _auction_id: AuctionId, _value: Self::Balance) -> DispatchResult {
		Ok(())
	}

	fn local_auction_bid_handler(
		_now: u64,
		_id: u64,
		_new_bid: (AccountId, Self::Balance),
		_last_bid: Option<(AccountId, Self::Balance)>,
		_social_currency_id: FungibleTokenId,
	) -> DispatchResult {
		Ok(())
	}

	fn collect_royalty_fee(
		_high_bid_price: &Self::Balance,
		_high_bidder: &AccountId,
		_asset_id: &(u32, u64),
		_social_currency_id: FungibleTokenId,
	) -> DispatchResult {
		Ok(())
	}
}

impl CheckAuctionItemHandler<Balance> for MockAuctionManager {
	fn check_item_in_auction(_item_id: ItemId<Balance>) -> bool {
		return false;
	}
}

parameter_types! {
	pub ClassMintingFee: Balance = 2;
	pub AssetMintingFee: Balance = 1;
	pub NftPalletId: PalletId = PalletId(*b"bit/bNFT");
	pub MetaverseNetworkTreasuryPalletId: PalletId = PalletId(*b"bit/trsy");
	pub MaxBatchTransfer: u32 = 3;
	pub MaxBatchMinting: u32 = 2000;
	pub MaxMetadata: u32 = 10;
}

impl pallet_nft::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PalletId = NftPalletId;
	type WeightInfo = ();
	type AuctionHandler = MockAuctionManager;
	type MaxBatchTransfer = MaxBatchTransfer;
	type MaxBatchMinting = MaxBatchMinting;
	type MaxMetadata = MaxMetadata;
	type MultiCurrency = Currencies;
	type MiningResourceId = MiningCurrencyId;
	type Treasury = MetaverseNetworkTreasuryPalletId;
	type AssetMintingFee = AssetMintingFee;
	type ClassMintingFee = ClassMintingFee;
	type StorageDepositFee = StorageDepositFee;
	type OffchainSignature = Signature;
	type OffchainPublic = AccountPublic;
}

parameter_types! {
	pub MaxClassMetadata: u32 = 1024;
}

impl orml_nft::Config for Runtime {
	type ClassId = u32;
	type TokenId = u64;
	type Currency = Balances;
	type ClassData = pallet_nft::NftClassData<Balance>;
	type TokenData = NftAssetData<Balance>;
	type MaxClassMetadata = MaxClassMetadata;
	type MaxTokenMetadata = MaxTokenMetadata;
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum ProxyType {
	Any,
	JustTransfer,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::JustTransfer => matches!(c, RuntimeCall::Balances(pallet_balances::Call::transfer { .. })),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		self == &ProxyType::Any || self == o
	}
}
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match *c {
			// Remark is used as a no-op call in the benchmarking
			RuntimeCall::System(SystemCall::remark { .. }) => true,
			RuntimeCall::System(_) => false,
			_ => true,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ConstU128<1>;
	type ProxyDepositFactor = ConstU128<1>;
	type MaxProxies = ConstU32<4>;
	type WeightInfo = ();
	type CallHasher = BlakeTwo256;
	type MaxPending = ConstU32<2>;
	type AnnouncementDepositBase = ConstU128<1>;
	type AnnouncementDepositFactor = ConstU128<1>;
}

pub type RewardModule = Pallet<Runtime>;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: currencies::{ Pallet, Storage, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Config<T>, Event<T>},
		OrmlNft: orml_nft::{Pallet, Storage, Config<T>},
		NFTModule: pallet_nft::{Pallet, Storage ,Call, Event<T>},
		Reward: reward::{Pallet, Storage ,Call, Event<T>},
		Proxy: pallet_proxy
	}
);

pub struct ExtBuilder {
	balances: Vec<(AccountId, FungibleTokenId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, mut balances: Vec<(AccountId, FungibleTokenId, Balance)>) -> Self {
		self.balances.append(&mut balances);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![
				(ALICE, 10000),
				(BOB, 20000),
				(CHARLIE, 2000),
				(DONNA, 100000),
				(EVA, 1000),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: vec![
				(ALICE, FungibleTokenId::MiningResource(0), 10000),
				(BOB, FungibleTokenId::MiningResource(0), 5000),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
	}
}

pub fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Runtime>::events()
		.pop()
		.expect("Event expected")
		.event
}
