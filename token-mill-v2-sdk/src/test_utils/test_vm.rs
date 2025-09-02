use litesvm::{
    LiteSVM,
    types::{FailedTransactionMetadata, TransactionMetadata},
};
use litesvm_token::{
    CreateAssociatedTokenAccountIdempotent, MintTo, SetAuthority, get_spl_account,
    spl_token::{
        self,
        instruction::AuthorityType,
        state::{Account, Mint},
    },
};
use solana_sdk::{
    clock::Clock, instruction::Instruction, message::Message, native_token::sol_str_to_lamports,
    program_pack::Pack, pubkey, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction::create_account, transaction::Transaction,
};

use super::constants::*;

pub fn get_vm(actors: Vec<Pubkey>) -> LiteSVM {
    let mut svm = LiteSVM::new()
        .with_sigverify(false)
        .with_blockhash_check(false)
        .with_transaction_history(0);

    for actor in actors.clone() {
        svm.airdrop(&actor, sol_str_to_lamports("10.0").unwrap())
            .unwrap();
    }

    svm.add_program_from_file(
        token_mill_v2_client::programs::TOKEN_MILL_V2_ID,
        "./src/test_utils/programs/token_mill_v2.so",
    )
    .unwrap();

    svm.add_program_from_file(METADATA_PROGRAM, "./src/test_utils/programs/metadata.so")
        .unwrap();

    set_clock(&mut svm, CLOCK);

    svm
}

pub fn execute_instructions(
    svm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    payer: &Pubkey,
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let transaction = Transaction::new_unsigned(Message::new(&instructions, Some(payer)));

    let result = svm.send_transaction(transaction);

    if let Some(err) = result.as_ref().err() {
        println!("\nTransaction failed: {}\n", err.err);
        println!("{}", err.meta.pretty_logs());
    }

    result
}

pub fn warp(svm: &mut LiteSVM, time: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += time;
    svm.set_sysvar(&clock);
}

fn set_clock(svm: &mut LiteSVM, clock_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = clock_timestamp;
    svm.set_sysvar(&clock);
}

pub fn create_tokens<const N: usize>(
    svm: &mut LiteSVM,
    addresses: [Pubkey; N],
    recipients: Vec<Pubkey>,
    future_recipients: Vec<Pubkey>,
    authority: Option<&Pubkey>,
) {
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), sol_str_to_lamports("10.0").unwrap())
        .unwrap();

    for address in addresses {
        create_mint(svm, &address, Some(&payer.pubkey()), Some(9));
    }

    create_atas(
        svm,
        addresses.to_vec(),
        recipients
            .iter()
            .chain(future_recipients.iter())
            .cloned()
            .collect(),
    );

    for token in &addresses {
        for recipient in &recipients {
            MintTo::new(
                svm,
                &payer,
                token,
                &get_ata(recipient, token),
                u64::MAX / (recipients.len() as u64 + 1),
            )
            .send()
            .unwrap();
        }
    }

    if let Some(authority) = authority {
        for token in &addresses {
            let set_authority_action =
                SetAuthority::new(svm, &payer, token, AuthorityType::MintTokens)
                    .new_authority(&authority);

            set_authority_action.owner(&payer).send().unwrap();
        }
    }
}

pub fn create_mint(
    svm: &mut LiteSVM,
    address: &Pubkey,
    authority: Option<&Pubkey>,
    decimals: Option<u8>,
) {
    let ix1 = create_account(
        &ALICE,
        address,
        svm.minimum_balance_for_rent_exemption(Mint::LEN),
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let ix2 = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        &address,
        authority.unwrap_or(&ALICE),
        None,
        decimals.unwrap_or(8),
    )
    .unwrap();

    execute_instructions(svm, vec![ix1, ix2], &ALICE).unwrap();
}

pub fn create_atas(svm: &mut LiteSVM, mints: Vec<Pubkey>, owners: Vec<Pubkey>) {
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), sol_str_to_lamports("10.0").unwrap())
        .unwrap();

    for mint in &mints {
        for owner in &owners {
            CreateAssociatedTokenAccountIdempotent::new(svm, &payer, mint)
                .owner(owner)
                .send()
                .unwrap();
        }
    }
}

pub fn get_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            &owner.to_bytes(),
            &spl_token::ID.to_bytes(),
            &mint.to_bytes(),
        ],
        &pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
    )
    .0
}

pub fn get_token_balances<const N: usize>(
    svm: &LiteSVM,
    owner: &Pubkey,
    mint: [&Pubkey; N],
) -> [u64; N] {
    mint.map(|mint| {
        get_spl_account::<Account>(&svm, &get_ata(owner, mint))
            .unwrap()
            .amount
    })
}

pub fn get_token_balance(svm: &LiteSVM, owner: &Pubkey, mint: &Pubkey) -> u64 {
    get_spl_account::<Account>(&svm, &get_ata(owner, mint))
        .unwrap()
        .amount
}

pub fn make_address(string: &str) -> Pubkey {
    assert!(
        string.len() <= 32,
        "{}",
        format!("\"{}\" too long to make an address !", string)
    );

    let mut array: [u8; 32] = [0; 32];

    // Iterate over each character in the input string
    for (index, byte) in string.bytes().enumerate() {
        array[index] = byte;
    }

    // Convert the array to a Pubkey
    Pubkey::new_from_array(array)
}
