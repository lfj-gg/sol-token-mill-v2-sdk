use solana_sdk::{pubkey, pubkey::Pubkey};

// Actors
pub const ALICE: Pubkey = pubkey!("7ZJRjoNm7DjokzybCnaMqJ9bNQWTPkW4BuackD6zr9PD");
pub const BOB: Pubkey = pubkey!("7dFTnr8mvkHEQABLrsnybnHAm84HhEAJdn4ZpMccEhHh");

pub const TOKEN_MINT_0: Pubkey = pubkey!("CDCD3xq4DN3kXx18Tdpx4FH4zFZ2ciCaGqDEErmLSSfu");
pub const TOKEN_MINT_1: Pubkey = pubkey!("G1RFVzDxJRSeYHE1KXExLVw1Avo7kaKwtLjLXLfpiXvg");
pub const CONFIG: Pubkey = pubkey!("D6qYhV5juiHXsqCa72iaKJWs2nAexJPnFA59764rAu93");
pub const MARKET: Pubkey = pubkey!("Hbb63RfKy5Dpba5W4xNkRcM1G67jaHpzHmvQe2Bck2Lq"); // PDA from TOKEN_MINT_0

pub const METADATA_PROGRAM: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

pub const PROTOCOL_FEE_SHARE: u32 = 400_000; // 40%
pub const FEE_UPDATE_COOLDOWN: u32 = 3_600; // 1 hour

// Market parameters
pub const MAX_SUPPLY: u64 = 1_000_000_000_000_000; // 1e9 * 1e6
pub const SUPPLY_AT_GRADUATION: u64 = 800_000_000_000_000; // 8e8 * 1e6
pub const SUPPLY_POOL_B: u64 = MAX_SUPPLY - SUPPLY_AT_GRADUATION;
pub const SQRT_PRICE_A: u128 = 419236029690706642379639606; // 2.8e-5
pub const SQRT_PRICE_B: u128 = 1544441212687274377713657485; // 3.8e-4
pub const FEE: u32 = 10_000; // 1%

// Test settings
pub const CLOCK: i64 = 200;
