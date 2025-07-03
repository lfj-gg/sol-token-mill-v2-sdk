# Token Mill V2

Solana token launcher from LFJ

## Bonding Curve

Token Mill V2 bonding curve is the combination of 2 Uni-V3 style pricing curves, called pool A and B in the code. Pool A will be in effect until 80% of the total supply is minted, and pool B for the rest, mimicking a graduation event. Those pools use two different virtual liquidity amounts, that will change the pricing dynamic when switching from one to the other.

![Price Curve](https://github.com/user-attachments/assets/f127c0b8-acf4-43c7-aed3-c9732db90fe2)

## King of the Mill

Token Mill V2 allows creators to redirect their share of swap fees to a fee pool, that will be used for a buyback program called King of the Mill (KotM). Every 30 minutes, fees accumulated will be used to buy some of the top tokens.

To opt in and out of the program, market creators can set the market `fee_reserve` to an account they control, or to `None`, which will send the fee to the config `creator_fee_pool` used for the KotM program. Setting the market fee reserve has a time cooldown, to avoid gaming the system.

## Components

### Program

#### Config Account

The config account holds various protocol parameters, including:
- Quote token - **Immutable**
- Market parameters - **Mutable**
- Fee parameters (protocol fee share, recipient) - **Mutable**

Config accounts can be created by anyone using the `create_config` instruction, but only markets created using the official Token Mill config will be indexed and shown on the website.

#### Market account

The market account holds all the informations required to perfom swaps.

The token created with the market is called *base token*, or *token 0* in the code. It will be traded against the *quote token*, or *token 1*, set on the config.

### Client

Rust accounts parsing and instruction builders are provided by the client in `client/src/generated`, automatically generated using [Codama](https://github.com/codama-idl/codama)