use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};

const GET_KEYPAIR_URL: &str = "https://sol-barn.tokenmill.xyz/v2/keypairs/available";
const SIGN_MARKET_CREATION_URL: &str =
    "https://sol-barn.tokenmill.xyz/v2/keypairs/sign-transaction";

pub fn get_vanity_address() -> Result<Pubkey> {
    let client = reqwest::blocking::ClientBuilder::new()
        .use_rustls_tls()
        .build()?;

    let result = client
        .get(GET_KEYPAIR_URL)
        .header("Content-Type", "application/json")
        .header("referer", "https://tokenmill.xyz")
        .send()?;

    let text = result.text()?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let pubkey_str = json["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("pubkey not found in response"))?;

    let pubkey = Pubkey::try_from(pubkey_str)?;

    Ok(pubkey)
}

pub fn sign_market_creation_with_vanity(tx: &mut Transaction) -> Result<()> {
    let serialized_tx = bincode::serialize(tx).unwrap();
    let serialized_tx_base58 = bs58::encode(&serialized_tx).into_string();

    let client = reqwest::blocking::ClientBuilder::new()
        .use_rustls_tls()
        .build()?;

    let result = client
        .post(SIGN_MARKET_CREATION_URL)
        .header("Content-Type", "application/json")
        .header("referer", "https://tokenmill.xyz")
        .body(format!("{{\"transaction\":\"{}\"}}", serialized_tx_base58))
        .send()?;

    let text = result.text()?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let signed_tx_base58 = json["transaction"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("transaction not found in response"))?;

    let signed_tx =
        bincode::deserialize::<Transaction>(&bs58::decode(signed_tx_base58).into_vec()?)?;

    tx.signatures = signed_tx.signatures;

    Ok(())
}
