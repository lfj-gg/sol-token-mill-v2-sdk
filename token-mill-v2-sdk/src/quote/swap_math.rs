use anyhow::Result;
use ruint::aliases::U256;

use crate::quote::math::{mul_div, mul_div_round_up};
use token_mill_v2_client::errors::TokenMillV2Error::*;

type GetAmountFn = fn(u128, u128, u128, bool) -> Result<u128>;

pub const MAX_FEE_U128: u128 = 1_000_000;
pub const SQRT_PRICE_SHIFT: usize = 96;

pub fn get_delta_amounts(
    sqrt_price: u128,
    target_sqrt_price: u128,
    liquidity: u128,
    delta_amount: i64,
    fee: u32,
) -> Result<(u128, u64, u64, u64)> {
    // Returns the new sqrt price, amount in, amount out and fee amount
    let (new_sqrt_price, amount_in, amount_out, fee_amount): (u128, u64, u64, u64);

    let zero_for_one = target_sqrt_price < sqrt_price;

    let (get_amount_in, get_amount_out): (GetAmountFn, GetAmountFn) = if zero_for_one {
        (get_amount_0, get_amount_1)
    } else {
        (get_amount_1, get_amount_0)
    };

    if delta_amount.is_positive() {
        let delta_amount = delta_amount.unsigned_abs();
        let amount_in_available =
            (u128::from(delta_amount) * (MAX_FEE_U128 - u128::from(fee))) / MAX_FEE_U128;

        // If the amount overflows, that means we won't be able to reach the target price
        // `max_amount_in` is set to `u128::MAX` so that it will always be bigger than `amount_in_available`
        let max_amount_in =
            get_amount_in(sqrt_price, target_sqrt_price, liquidity, true).or_else(|err| {
                if err
                    .downcast_ref::<token_mill_v2_client::errors::TokenMillV2Error>()
                    .map_or(false, |e| *e == AmountOverflow)
                {
                    Ok(u128::MAX)
                } else {
                    Err(err)
                }
            })?;

        if max_amount_in > amount_in_available {
            new_sqrt_price = if zero_for_one {
                get_next_sqrt_ratio_from_amount_0(
                    sqrt_price,
                    liquidity,
                    i64::try_from(amount_in_available).map_err(|_| AmountInOverflow)?,
                )?
            } else {
                get_next_sqrt_ratio_from_amount_1(
                    sqrt_price,
                    liquidity,
                    i64::try_from(amount_in_available).map_err(|_| AmountInOverflow)?,
                )?
            };

            amount_in = get_amount_in(sqrt_price, new_sqrt_price, liquidity, true)?
                .try_into()
                .map_err(|_| AmountInOverflow)?;
            fee_amount = delta_amount - amount_in;
        } else {
            new_sqrt_price = target_sqrt_price;
            // Safe cast as max_amount_in <= amount_in_available
            amount_in = max_amount_in as u64;

            fee_amount = u64::try_from(
                (max_amount_in * u128::from(fee)).div_ceil(MAX_FEE_U128 - u128::from(fee)),
            )
            .map_err(|_| FeeAmountOverflow)?;
        }

        amount_out = get_amount_out(sqrt_price, new_sqrt_price, liquidity, false)?
            .try_into()
            .map_err(|_| AmountOutOverflow)?;
    } else {
        if delta_amount == 0 {
            return Ok((sqrt_price, 0, 0, 0));
        };

        let amount_out_to_fill = delta_amount.unsigned_abs();

        // If the amount overflows, that means we won't be able to reach the target price
        // `max_amount_out` is set to `u128::MAX` so that it will always be bigger than `amount_out_to_fill`
        let max_amount_out = get_amount_out(sqrt_price, target_sqrt_price, liquidity, false)
            .or_else(|err| {
                if err
                    .downcast_ref::<token_mill_v2_client::errors::TokenMillV2Error>()
                    .map_or(false, |e| *e == AmountOverflow)
                {
                    Ok(u128::MAX)
                } else {
                    Err(err)
                }
            })?;

        if max_amount_out > amount_out_to_fill.into() {
            new_sqrt_price = if zero_for_one {
                get_next_sqrt_ratio_from_amount_1(sqrt_price, liquidity, delta_amount)?
            } else {
                get_next_sqrt_ratio_from_amount_0(sqrt_price, liquidity, delta_amount)?
            };
            amount_out = amount_out_to_fill;
        } else {
            new_sqrt_price = target_sqrt_price;
            // Safe cast as max_amount_out <= amount_out_to_fill
            amount_out = max_amount_out as u64;
        }

        amount_in = get_amount_in(sqrt_price, new_sqrt_price, liquidity, true)?
            .try_into()
            .map_err(|_| AmountInOverflow)?;

        fee_amount = u64::try_from(
            (u128::from(amount_in) * u128::from(fee)).div_ceil(MAX_FEE_U128 - u128::from(fee)),
        )
        .map_err(|_| FeeAmountOverflow)?;
    }

    Ok((new_sqrt_price, amount_in, amount_out, fee_amount))
}

// Returns an u128, as it could be used with an "infinite" sqrt price limit
// Amount is downcasted safely inside `get_delta_amounts` if necessary
pub fn get_amount_0(
    sqrt_price_a: u128,
    sqrt_price_b: u128,
    liquidity: u128,
    adding: bool,
) -> Result<u128> {
    let (sqrt_price_a, sqrt_price_b) = if sqrt_price_a < sqrt_price_b {
        (sqrt_price_a, sqrt_price_b)
    } else {
        (sqrt_price_b, sqrt_price_a)
    };

    if adding {
        mul_div_round_up(
            U256::from(liquidity).saturating_shl(SQRT_PRICE_SHIFT),
            U256::from(sqrt_price_b - sqrt_price_a),
            U256::from(sqrt_price_a) * U256::from(sqrt_price_b),
        )
    } else {
        mul_div(
            U256::from(liquidity).saturating_shl(SQRT_PRICE_SHIFT),
            U256::from(sqrt_price_b - sqrt_price_a),
            U256::from(sqrt_price_a) * U256::from(sqrt_price_b),
        )
    }
}

// Returns an u128, as it could be used with an "infinite" sqrt price limit
// Amount is downcasted safely inside `get_delta_amounts` if necessary
pub fn get_amount_1(
    sqrt_price_a: u128,
    sqrt_price_b: u128,
    liquidity: u128,
    adding: bool,
) -> Result<u128> {
    let (sqrt_price_a, sqrt_price_b) = if sqrt_price_a < sqrt_price_b {
        (sqrt_price_a, sqrt_price_b)
    } else {
        (sqrt_price_b, sqrt_price_a)
    };

    if adding {
        (U256::from(liquidity) * U256::from(sqrt_price_b - sqrt_price_a))
            .div_ceil(U256::from(2u128.pow(SQRT_PRICE_SHIFT as u32)))
            .try_into()
            .map_err(|_| AmountOverflow.into())
    } else {
        ((U256::from(liquidity) * U256::from(sqrt_price_b - sqrt_price_a))
            .wrapping_shr(SQRT_PRICE_SHIFT))
        .try_into()
        .map_err(|_| AmountOverflow.into())
    }
}

pub fn get_next_sqrt_ratio_from_amount_0(
    sqrt_price: u128,
    liquidity: u128,
    amount_0: i64,
) -> Result<u128> {
    if amount_0 == 0 {
        return Ok(sqrt_price);
    }

    let liquidity = U256::from(liquidity).saturating_shl(SQRT_PRICE_SHIFT);

    let denominator = match amount_0.is_positive() {
        true => liquidity + U256::from(amount_0) * U256::from(sqrt_price),
        false => liquidity
            .checked_sub(U256::from(amount_0.abs()) * U256::from(sqrt_price))
            .ok_or(LiquidityOverflow0)?,
    };

    mul_div_round_up(liquidity, U256::from(sqrt_price), denominator)
}

pub fn get_next_sqrt_ratio_from_amount_1(
    sqrt_price: u128,
    liquidity: u128,
    amount_1: i64,
) -> Result<u128> {
    let numerator = match amount_1.is_positive() {
        true => {
            U256::from(sqrt_price) * U256::from(liquidity)
                + U256::from(amount_1).saturating_shl(SQRT_PRICE_SHIFT)
        }
        false => (U256::from(sqrt_price) * U256::from(liquidity))
            .checked_sub(U256::from(amount_1.abs()).saturating_shl(SQRT_PRICE_SHIFT))
            .ok_or(LiquidityOverflow1)?,
    };

    let sqrt_price_next = numerator / U256::from(liquidity);

    sqrt_price_next.try_into().map_err(|_| PriceOverflow.into())
}
