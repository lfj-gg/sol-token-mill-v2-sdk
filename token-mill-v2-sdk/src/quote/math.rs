use anyhow::Result;
use ruint::aliases::{U256, U512};
use token_mill_v2_client::errors::TokenMillV2Error;

pub fn mul_div(x: U256, y: U256, denominator: U256) -> Result<u128> {
    if denominator.is_zero() {
        return Err(TokenMillV2Error::DivisionByZero.into());
    }

    let x = U512::from(x);
    let y = U512::from(y);
    let denominator = U512::from(denominator);

    let prod = x.wrapping_mul(y);

    let (quotient, _) = prod.div_rem(denominator);

    quotient
        .try_into()
        .map_err(|_| TokenMillV2Error::AmountOverflow.into())
}

pub fn mul_div_round_up(x: U256, y: U256, denominator: U256) -> Result<u128> {
    let result = mul_div(x, y, denominator)?;

    if (x % denominator).is_zero() {
        Ok(result)
    } else {
        result
            .checked_add(1)
            .ok_or(TokenMillV2Error::AmountOverflow.into())
    }
}
