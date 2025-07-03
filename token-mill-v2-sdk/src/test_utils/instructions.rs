use litesvm::{
    LiteSVM,
    types::{FailedTransactionMetadata, TransactionMetadata},
};
use num_traits::FromPrimitive;
use solana_sdk::{instruction::InstructionError, pubkey::Pubkey, transaction::TransactionError};
use token_mill_v2_client::{errors::TokenMillV2Error, instructions::*, types::MarketSettingsInput};

use super::{constants::*, test_vm::*};

pub fn get_vm_and_create_market() -> LiteSVM {
    let mut svm = get_vm(vec![ALICE, BOB]);

    create_tokens(&mut svm, [TOKEN_MINT_1], vec![ALICE, BOB], vec![], None);

    execute_instructions(
        &mut svm,
        vec![
            get_create_config_ix_builder().instruction(),
            get_market_creation_ix_builder().instruction(),
        ],
        &ALICE,
    )
    .unwrap();

    create_atas(&mut svm, vec![TOKEN_MINT_0], vec![ALICE, BOB]);

    svm
}

pub fn parse_error(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
) -> Result<TokenMillV2Error, TransactionError> {
    let result = result.unwrap_err();

    match result.err {
        TransactionError::InstructionError(_, InstructionError::Custom(error_code)) => {
            Ok(TokenMillV2Error::from_u32(error_code).unwrap())
        }
        _ => Err(result.err),
    }
}

pub fn get_create_config_ix_builder() -> CreateConfigBuilder {
    let mut create_config_builder = CreateConfigBuilder::new();

    create_config_builder
        .token_mill_config(CONFIG)
        .quote_token_mint(TOKEN_MINT_1)
        .protocol_fee_share(PROTOCOL_FEE_SHARE)
        .protocol_fee_token_account(get_ata(&BOB, &TOKEN_MINT_1))
        .kotm_fee_token_account(get_ata(&BOB, &TOKEN_MINT_1))
        .fee_recipient_change_cooldown(FEE_UPDATE_COOLDOWN)
        .market_settings(MarketSettingsInput {
            max_supply: MAX_SUPPLY,
            supply_at_graduation: SUPPLY_AT_GRADUATION,
            sqrt_price_a_x96: SQRT_PRICE_A,
            sqrt_price_b_x96: SQRT_PRICE_B,
            fee: FEE,
        })
        .admin(ALICE);

    create_config_builder
}

pub fn get_market_creation_ix_builder() -> CreateMarketBuilder {
    let mut create_market_builder = CreateMarketBuilder::new();

    let token_mint_0_metadata = Pubkey::find_program_address(
        &[
            "metadata".as_bytes(),
            &METADATA_PROGRAM.to_bytes(),
            &TOKEN_MINT_0.to_bytes(),
        ],
        &METADATA_PROGRAM,
    )
    .0;

    create_market_builder
        .token_mill_config(CONFIG)
        .name("Test Market".to_string())
        .uri("uri.url".to_string())
        .symbol("TEST".to_string())
        .token_mint0(TOKEN_MINT_0)
        .token0_metadata(token_mint_0_metadata)
        .token_mint1(TOKEN_MINT_1)
        .market(MARKET)
        .market_reserve0(get_ata(&MARKET, &TOKEN_MINT_0))
        .market_reserve1(get_ata(&MARKET, &TOKEN_MINT_1))
        .creator(ALICE);

    create_market_builder
}

pub fn get_swap_ix_builder() -> SwapBuilder {
    let mut swap_builder = SwapBuilder::new();

    swap_builder
        .config(CONFIG)
        .market(MARKET)
        .market_reserve0(get_ata(&MARKET, &TOKEN_MINT_0))
        .market_reserve1(get_ata(&MARKET, &TOKEN_MINT_1))
        .fee_reserve(get_ata(&BOB, &TOKEN_MINT_1))
        .protocol_fee_reserve(get_ata(&BOB, &TOKEN_MINT_1))
        .creator_fee_pool(get_ata(&BOB, &TOKEN_MINT_1))
        .user_token_account0(get_ata(&ALICE, &TOKEN_MINT_0))
        .user_token_account1(get_ata(&ALICE, &TOKEN_MINT_1))
        .user(ALICE);

    swap_builder
}

pub fn get_swap_with_price_limit_ix_builder() -> SwapWithPriceLimitBuilder {
    let mut swap_with_price_limit_builder = SwapWithPriceLimitBuilder::new();

    swap_with_price_limit_builder
        .config(CONFIG)
        .market(MARKET)
        .market_reserve0(get_ata(&MARKET, &TOKEN_MINT_0))
        .market_reserve1(get_ata(&MARKET, &TOKEN_MINT_1))
        .fee_reserve(get_ata(&BOB, &TOKEN_MINT_1))
        .protocol_fee_reserve(get_ata(&BOB, &TOKEN_MINT_1))
        .creator_fee_pool(get_ata(&BOB, &TOKEN_MINT_1))
        .user_token_account0(get_ata(&ALICE, &TOKEN_MINT_0))
        .user_token_account1(get_ata(&ALICE, &TOKEN_MINT_1))
        .user(ALICE);

    swap_with_price_limit_builder
}
