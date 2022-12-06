use core_primitives::{Attributes, CollectionType, NftMetadata, TokenType};
use frame_support::pallet_prelude::Get;
use frame_support::traits::{Currency, OriginTrait};
use frame_system::RawOrigin;
use orml_traits::{BasicCurrency, MultiCurrency};
use pallet_evm::{
	AddressMapping, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileHandle, PrecompileOutput,
	PrecompileResult, PrecompileSet,
};
use sp_core::{H160, U256};
use sp_runtime::traits::{AccountIdConversion, Dispatchable};
use sp_runtime::Perbill;
use sp_std::{marker::PhantomData, prelude::*};

use precompile_utils::data::{Address, EvmData, EvmDataWriter};
use precompile_utils::handle::PrecompileHandleExt;
use precompile_utils::modifier::FunctionModifier;
use precompile_utils::prelude::RuntimeHelper;
use precompile_utils::{succeed, EvmResult};
use primitives::evm::{Erc20Mapping, Output};
use primitives::{evm, Balance, ClassId, GroupCollectionId, TokenId};

#[precompile_utils_macro::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	//GetNftAssetInfo = "getNftAssetInfo()",
	//GetAssetOwner = "getAssetOwner()",
	GetClassFundBalance = "geetClassFundBalance()",
	CreateClass = "createClass()",
	MintNfts = "mintNfts()",
	TransferNft = "transferNft()",
	BurnNft = "burnNft()",
	WithdrawFromClassFund = "withdrawFromClassFund()",
}

//Alias for the Balance type for the provided Runtime and Instance.
//pub type BalanceOf<Runtime> = <<Runtime as nft:Config>::Currency as Trait>::Balance;

//Alias for the ClassId type for the provided Runtime and Instance.
pub type ClassIdOf<Runtime> = <Runtime as orml_nft::Config>::ClassId;

//Alias for the TokenId type for the provided Runtime and Instance.
pub type TokenIdOf<Runtime> = <Runtime as orml_nft::Config>::TokenId;

/// The `Nft` impl precompile.
///
///
/// `input` data starts with `action`, `class_id`, and `token_id`.
///
///
/// Actions:
/// - Get NFT asset info.
/// - Get asset owner.
/// - Get class fund balance.
/// - Create class.
/// - Mint NFTs.
/// - Transfer NFT. Rest `input` bytes: `from`, `to`, and (`class_id`, `token_id`).
/// - Burn NFT. Rest `input` bytes: `from`, and (`class_id`, `token_id`).
/// - Withdraw from class fund. Rest `input` bytes: `(`class_id`, `token_id`).
pub struct NftPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for NftPrecompile<Runtime>
where
	Runtime: nft::Config + pallet_evm::Config + frame_system::Config,
	Runtime: Erc20Mapping,
	U256: From<
		<<Runtime as nft::Config>::Currency as frame_support::traits::Currency<
			<Runtime as frame_system::Config>::AccountId,
		>>::Balance,
	>,
	ClassIdOf<Runtime>: TryFrom<U256> + Into<ClassId> + EvmData,
	TokenIdOf<Runtime>: TryFrom<U256> + Into<TokenId> + EvmData,
	<<Runtime as frame_system::Config>::Call as Dispatchable>::Origin: OriginTrait,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let result = {
			let selector = match handle.read_selector() {
				Ok(selector) => selector,
				Err(e) => return Err(e),
			};

			if let Err(err) = handle.check_function_modifier(match selector {
				Action::CreateClass
				| Action::MintNfts
				| Action::BurnNft
				| Action::TransferNft
				| Action::WithdrawFromClassFund => FunctionModifier::NonPayable,
				_ => FunctionModifier::View,
			}) {
				return Err(err);
			}

			match selector {
				// Local and Foreign common
				//Action::GetNftAssetInfo => Self::nft_info(handle),
				//Action::GetAssetOwner => Self::nft_info(handle),
				Action::GetClassFundBalance => Self::class_fund_balance(handle),
				Action::CreateClass => Self::create_class(handle),
				Action::MintNfts => Self::mint_nfts(handle),
				Action::TransferNft => Self::transfer_nft(handle),
				Action::BurnNft => Self::burn_nft(handle),
				Action::WithdrawFromClassFund => Self::withdraw_class_funds(handle),
			}
		};
		Err(PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: "invalid nft action".into(),
		})
	}
}

impl<Runtime> NftPrecompile<Runtime>
where
	Runtime: nft::Config + pallet_evm::Config + frame_system::Config,
	U256: From<
		<<Runtime as nft::Config>::Currency as frame_support::traits::Currency<
			<Runtime as frame_system::Config>::AccountId,
		>>::Balance,
	>,
	ClassIdOf<Runtime>: TryFrom<U256> + Into<ClassId> + EvmData,
	TokenIdOf<Runtime>: TryFrom<U256> + Into<TokenId> + EvmData,
	// BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + EvmData,
{
	/*
		fn nft_info(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
			handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

			// Parse input of 2 (class_id, token_id)
			let mut input = handle.read_input()?;
			input.expect_arguments(2)?;

			let class_id = input.read::<ClassIdOf<Runtime>>()?.into();
			let token_id = input.read::<TokenIdOf<Runtime>>()?.into();

			// Fetch info
			let nft_info_result = <nft::Pallet<Runtime>>::NftModule::<Runtime>::tokens(class_id, token_id);

			match nft_info_result
			{
				Some(nft_info) =>  {
					log::debug!(target: "evm", "Nft asset info: {:?}", nft_info);
					let encoded = Output::encode_bytes(&nft_info.data);
					// Build output.
					Ok(succeed(encoded))
				}
				None => {
					Err(PrecompileFailure::Revert {
						exit_status: ExitRevert::Reverted,
						output: "invalid nft asset".into(),
					})
				}
			}
		}
	*/
	fn class_fund_balance(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (class_id)
		let mut input = handle.read_input()?;
		input.expect_arguments(1)?;

		let class_id = input.read::<ClassIdOf<Runtime>>()?.into();

		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(owner);

		let class_treasury = <Runtime as nft::Config>::Treasury::get().into_sub_account_truncating(class_id);

		// Fetch info
		let balance = <Runtime as nft::Config>::Currency::free_balance(&class_treasury);

		log::debug!(target: "evm", "class: ({:?}, ) fund balance: {:?}", class_id, balance);

		let encoded = Output::encode_uint(balance);
		// Build output.
		Ok(succeed(encoded))
	}

	fn create_class(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (owner)
		let mut input = handle.read_input()?;
		input.expect_arguments(8)?;

		// Build call info
		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(owner);

		let class_metadata: NftMetadata = input.read::<NftMetadata>()?.into();
		let class_attributes: Attributes = input.read::<Attributes>()?.into();
		let class_collection_id: GroupCollectionId = input.read::<GroupCollectionId>()?.into();

		let class_token_type: TokenType = input.read::<TokenType>()?.into();
		let class_collection_type: CollectionType = input.read::<CollectionType>()?.into();
		let class_royalty_fee: Perbill = input.read::<Perbill>()?.into();
		let class_mint_limit: Option<u32> = input.read::<Option<u32>>()?.into();

		log::debug!(target: "evm", "create class for: {:?}", who);

		<nft::Pallet<Runtime>>::create_class(
			RawOrigin::Signed(who).into(),
			class_metadata,
			class_attributes,
			class_collection_id,
			class_token_type,
			class_collection_type,
			class_royalty_fee,
			class_mint_limit,
		)
		.map_err(|e| PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: Into::<&str>::into(e).as_bytes().to_vec(),
		})?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn mint_nfts(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (owner)
		let mut input = handle.read_input()?;
		input.expect_arguments(5)?;

		// Build call info
		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(owner);

		let nft_class_id = input.read::<ClassIdOf<Runtime>>()?.into();
		let nft_metadata: NftMetadata = input.read::<NftMetadata>()?.into();
		let nft_attributes: Attributes = input.read::<Attributes>()?.into();
		let nft_quantity: u32 = input.read::<u32>()?.into();

		log::debug!(target: "evm", "create class for: {:?}", who);

		<nft::Pallet<Runtime>>::mint(
			RawOrigin::Signed(who).into(),
			nft_class_id.into(),
			nft_metadata,
			nft_attributes,
			nft_quantity,
		)
		.map_err(|e| PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: Into::<&str>::into(e).as_bytes().to_vec(),
		})?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn transfer_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (owner)
		let mut input = handle.read_input()?;
		input.expect_arguments(3)?;

		// Build call info
		let to: H160 = input.read::<Address>()?.into();

		let class_id = input.read::<ClassIdOf<Runtime>>()?.into();
		let token_id = input.read::<TokenIdOf<Runtime>>()?.into();

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let to = Runtime::AddressMapping::into_account_id(to);

		log::debug!(target: "evm", "nft transfer from: {:?}, to: {:?}, token: ({:?}, {:?})", origin, to, class_id, token_id);

		<nft::Pallet<Runtime>>::transfer(RawOrigin::Signed(origin).into(), to, (class_id.into(), token_id.into()))
			.map_err(|e| PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: Into::<&str>::into(e).as_bytes().to_vec(),
			})?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn burn_nft(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (owner)
		let mut input = handle.read_input()?;
		input.expect_arguments(3)?;

		// Build call info
		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(owner);
		let class_id = input.read::<ClassIdOf<Runtime>>()?.into();
		let token_id = input.read::<TokenIdOf<Runtime>>()?.into();

		log::debug!(target: "evm", "burn nft asset: ({:?}, {:?})", class_id, token_id);

		<nft::Pallet<Runtime>>::burn(RawOrigin::Signed(who).into(), (class_id.into(), token_id.into())).map_err(
			|e| PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: Into::<&str>::into(e).as_bytes().to_vec(),
			},
		)?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}

	fn withdraw_class_funds(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		// Parse input of index 1 (class_id)
		let mut input = handle.read_input()?;
		input.expect_arguments(2)?;

		let class_id = input.read::<ClassIdOf<Runtime>>()?.into();

		// Build call info
		let owner: H160 = input.read::<Address>()?.into();
		let who: Runtime::AccountId = Runtime::AddressMapping::into_account_id(owner);

		log::debug!(target: "evm", "withdraw funds from class {:?} fund", class_id);

		<nft::Pallet<Runtime>>::withdraw_funds_from_class_fund(RawOrigin::Signed(who).into(), class_id.into())
			.map_err(|e| PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: Into::<&str>::into(e).as_bytes().to_vec(),
			})?;

		// Build output.
		Ok(succeed(EvmDataWriter::new().write(true).build()))
	}
}
