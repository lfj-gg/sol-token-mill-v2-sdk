use anyhow::Result;
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams, try_get_account_data,
};
use solana_sdk::pubkey::Pubkey;

use crate::quote::quote;
use token_mill_v2_client::{
    accounts::{Market, TokenMillConfig},
    errors::TokenMillV2Error,
    instructions::SwapBuilder,
    types::SwapParameters,
};

#[derive(Debug, Clone)]
pub struct TokenMillV2Amm {
    key: Pubkey,
    label: String,
    program_id: Pubkey,
    market_state: Market,
    protocol_fee_reserve: Pubkey,
    creator_fee_pool: Pubkey,
}

impl Amm for TokenMillV2Amm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        let data_slice: &[u8] = &keyed_account.account.data;
        let state = Market::from_bytes(data_slice)?;

        let label = "Token Mill V2".to_string();

        Ok(Self {
            key: keyed_account.key,
            label,
            program_id: keyed_account.account.owner,
            market_state: state,
            protocol_fee_reserve: Pubkey::default(), // Placeholder, will be updated in `update`
            creator_fee_pool: Pubkey::default(),     // Placeholder, will be updated in `update`
        })
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn label(&self) -> String {
        self.label.clone()
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.key, self.market_state.config]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        // Market
        let account = try_get_account_data(account_map, &self.key)?;
        let mut data_slice: &[u8] = &account;
        let market = Market::from_bytes(&mut data_slice)?;
        self.market_state = market;

        // Config
        let account = try_get_account_data(account_map, &self.market_state.config)?;
        let mut data_slice: &[u8] = &account;
        let config = TokenMillConfig::from_bytes(&mut data_slice)?;
        self.protocol_fee_reserve = config.protocol_fee_reserve;
        self.creator_fee_pool = config.creator_fee_pool;

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        let QuoteParams {
            amount,
            input_mint,
            swap_mode,
            ..
        } = quote_params;

        let market = &self.market_state;
        let zero_for_one = input_mint == &market.token_mint0;
        let amount_i64 = i64::try_from(*amount).map_err(|_| TokenMillV2Error::AmountOverflow)?;
        let delta_amount = if *swap_mode == SwapMode::ExactIn {
            amount_i64
        } else {
            -amount_i64
        };
        let sqrt_price_limit = if zero_for_one {
            market.settings.sqrt_price_a_x96
        } else {
            u128::MAX / 2
        };

        let result = quote(market, zero_for_one, delta_amount, sqrt_price_limit)?;

        Ok(Quote {
            in_amount: result.amount_in,
            out_amount: result.amount_out,
            fee_amount: result.fee_amount_token_1,
            fee_mint: self.market_state.token_mint1,
            ..Default::default()
        })
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![self.market_state.token_mint0, self.market_state.token_mint1]
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let zero_for_one = swap_params.source_mint == self.market_state.token_mint0;

        let (user_reserve_0, user_reserve_1) = match zero_for_one {
            true => (
                swap_params.source_token_account,
                swap_params.destination_token_account,
            ),
            false => (
                swap_params.destination_token_account,
                swap_params.source_token_account,
            ),
        };

        let mut swap_ix_builder = SwapBuilder::new();
        swap_ix_builder
            .swap_parameters(SwapParameters::BuyExactIn(0, 0)) // Dummy parameters, is required by the SwapBuilder to compile the instruction
            .config(self.market_state.config)
            .market(self.key)
            .market_reserve0(self.market_state.reserve0)
            .market_reserve1(self.market_state.reserve1)
            .fee_reserve(
                self.market_state
                    .fee_reserve
                    .unwrap_or(self.creator_fee_pool),
            )
            .protocol_fee_reserve(self.protocol_fee_reserve)
            .creator_fee_pool(self.creator_fee_pool)
            .user_token_account0(user_reserve_0)
            .user_token_account1(user_reserve_1)
            .user(swap_params.token_transfer_authority);

        Ok(SwapAndAccountMetas {
            swap: Swap::TokenSwap,
            account_metas: swap_ix_builder.instruction().accounts,
        })
    }

    fn supports_exact_out(&self) -> bool {
        true
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn get_accounts_len(&self) -> usize {
        14
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use borsh::BorshDeserialize;
    use jupiter_amm_interface::ClockRef;
    use solana_sdk::{clock::Clock, instruction::Instruction, native_token::sol_str_to_lamports};
    use token_mill_v2_client::{
        instructions::SwapInstructionData,
        types::{SwapParameters, SwapResult},
    };

    use crate::test_utils::{
        constants::{ALICE, CONFIG, MARKET, TOKEN_MINT_0, TOKEN_MINT_1},
        instructions::get_vm_and_create_market,
        test_vm::{execute_instructions, get_ata},
    };

    use super::*;

    #[test]
    fn swap() {
        let mut vm = get_vm_and_create_market();

        let market_keyed_account = KeyedAccount {
            key: MARKET,
            account: vm.get_account(&MARKET).unwrap(),
            params: None,
        };
        let mut amm = TokenMillV2Amm::from_keyed_account(
            &market_keyed_account,
            &AmmContext {
                clock_ref: ClockRef::from(vm.get_sysvar::<Clock>()),
            },
        )
        .unwrap();

        let mut account_map: AccountMap = HashMap::with_hasher(Default::default());
        account_map.insert(MARKET, vm.get_account(&MARKET).unwrap());
        account_map.insert(CONFIG, vm.get_account(&CONFIG).unwrap());

        amm.update(&account_map).unwrap();

        let amount_in = sol_str_to_lamports("1.0").unwrap();
        let min_amount_out = 0;

        let quote = amm
            .quote(&QuoteParams {
                amount: amount_in,
                input_mint: TOKEN_MINT_1,
                output_mint: TOKEN_MINT_0,
                swap_mode: SwapMode::ExactIn,
            })
            .unwrap();

        assert_eq!(quote.fee_mint, TOKEN_MINT_1);

        let swap_parameters = SwapParameters::BuyExactIn(amount_in, min_amount_out);

        let mut swap_data = borsh::to_vec(&SwapInstructionData::new()).unwrap();
        swap_data.append(&mut borsh::to_vec(&swap_parameters).unwrap());

        let swap_accounts = amm
            .get_swap_and_account_metas(&SwapParams {
                swap_mode: SwapMode::ExactIn,
                in_amount: amount_in,
                out_amount: min_amount_out,
                source_mint: TOKEN_MINT_1,
                destination_mint: TOKEN_MINT_0,
                source_token_account: get_ata(&ALICE, &TOKEN_MINT_1),
                destination_token_account: get_ata(&ALICE, &TOKEN_MINT_0),
                token_transfer_authority: ALICE,
                quote_mint_to_referrer: None,
                jupiter_program_id: &Pubkey::default(),
                missing_dynamic_accounts_as_default: false,
            })
            .unwrap();

        let instruction = Instruction {
            data: swap_data,
            accounts: swap_accounts.account_metas,
            program_id: amm.program_id(),
        };

        let result = execute_instructions(&mut vm, vec![instruction], &ALICE).unwrap();
        let result = SwapResult::try_from_slice(&result.return_data.data).unwrap();

        assert_eq!(result.amount_in, quote.in_amount);
        assert_eq!(result.amount_out, quote.out_amount);
        assert_eq!(result.fee_amount_token1, quote.fee_amount);
    }

    #[test]
    fn get_account_len() {
        let dummy = TokenMillV2Amm {
            key: Pubkey::default(),
            label: "test".to_string(),
            program_id: Pubkey::default(),
            market_state: unsafe { std::mem::zeroed() },
            protocol_fee_reserve: Pubkey::default(),
            creator_fee_pool: Pubkey::default(),
        };

        assert_eq!(
            dummy.get_accounts_len(),
            dummy
                .get_swap_and_account_metas(&SwapParams {
                    swap_mode: SwapMode::ExactIn,
                    in_amount: 0,
                    out_amount: 0,
                    source_mint: Pubkey::default(),
                    destination_mint: Pubkey::default(),
                    source_token_account: Pubkey::default(),
                    destination_token_account: Pubkey::default(),
                    token_transfer_authority: Pubkey::default(),
                    quote_mint_to_referrer: None,
                    jupiter_program_id: &Pubkey::default(),
                    missing_dynamic_accounts_as_default: false
                })
                .unwrap()
                .account_metas
                .len()
        );
    }
}
