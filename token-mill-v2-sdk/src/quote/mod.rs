use anyhow::Result;

use crate::quote::swap_math::get_delta_amounts;
use token_mill_v2_client::accounts::Market;

mod math;
mod swap_math;

#[derive(Debug, Clone)]
pub struct Quote {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount_token_in: u64,
    pub fee_amount_token_1: u64,
    pub next_sqrt_price: u128,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    A,
    B,
}

pub fn quote(
    market: &Market,
    zero_for_one: bool,
    delta_amount: i64,
    sqrt_price_limit: u128,
) -> Result<Quote> {
    let (mut next_sqrt_price, amount_in, amount_out, fee_amount_token_in) =
        get_delta_amounts_from_dual_pool(
            market,
            market.sqrt_price_x96,
            zero_for_one,
            delta_amount,
            sqrt_price_limit,
            market.settings.fee,
        )?;

    // Get fee as token 1
    let fee_amount_token_1 = if zero_for_one {
        let (sqrt_price_after_fee_swap, _, fee_amount, _) = get_delta_amounts_from_dual_pool(
            market,
            next_sqrt_price,
            true,
            i64::try_from(fee_amount_token_in)?,
            market.settings.sqrt_price_a_x96,
            0,
        )?;

        next_sqrt_price = sqrt_price_after_fee_swap;

        fee_amount
    } else {
        fee_amount_token_in
    };

    Ok(Quote {
        amount_in,
        amount_out,
        fee_amount_token_in,
        fee_amount_token_1,
        next_sqrt_price,
    })
}

fn get_delta_amounts_from_dual_pool(
    market: &Market,
    current_sqrt_price: u128,
    zero_for_one: bool,
    mut delta_amount: i64,
    sqrt_price_limit: u128,
    fee: u32,
) -> Result<(u128, u64, u64, u64)> {
    let phase = if current_sqrt_price < market.settings.sqrt_price_b_x96 {
        Phase::A
    } else {
        Phase::B
    };

    let (first_l, second_l) = match phase {
        Phase::A => (market.settings.liquidity_a, market.settings.liquidity_b),
        Phase::B => (market.settings.liquidity_b, market.settings.liquidity_a),
    };

    // First pool
    let first_sqrt_price_target = if zero_for_one == false {
        if phase == Phase::A {
            sqrt_price_limit.min(market.settings.sqrt_price_b_x96)
        } else {
            sqrt_price_limit
        }
    } else if phase == Phase::A {
        sqrt_price_limit
    } else {
        sqrt_price_limit.max(market.settings.sqrt_price_b_x96)
    };

    let (mut new_sqrt_price, mut amount_in, mut amount_out, mut fee_amount) = get_delta_amounts(
        current_sqrt_price,
        first_sqrt_price_target,
        first_l,
        delta_amount,
        fee,
    )?;

    if delta_amount.is_positive() {
        // Safe cast
        delta_amount -= (amount_in + fee_amount) as i64;
    } else {
        // Safe cast
        delta_amount += amount_out as i64;
    }

    // Second pool
    if delta_amount != 0 && new_sqrt_price != sqrt_price_limit {
        let (additional_amount_in, additional_amount_out, additional_fee_amount);

        (
            new_sqrt_price,
            additional_amount_in,
            additional_amount_out,
            additional_fee_amount,
        ) = get_delta_amounts(
            new_sqrt_price,
            sqrt_price_limit,
            second_l,
            delta_amount,
            fee,
        )?;

        amount_in += additional_amount_in;
        amount_out += additional_amount_out;
        fee_amount += additional_fee_amount;
    }

    Ok((
        new_sqrt_price,
        amount_in + fee_amount,
        amount_out,
        fee_amount,
    ))
}
