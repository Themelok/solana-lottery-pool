use borsh::{to_vec as borsh_to_vec, BorshSerialize};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::program::id as system_program_id;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_associated_token_account_interface::program::ID as associated_token_program_id;
use spl_token_interface::ID as token_program_id;
use std::str::FromStr;

// Program ID (must match the one in lib.rs)
const PROGRAM_ID: &str = "DmvwV2RNhdELbxn1nx5LVi6RexqKGDBoLJdWY9SGHjmP";

// USDC Mint on Devnet
const USDC_MINT_DEVNET: &str = "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";

// ============================================================================
// Instruction Discriminators
// ============================================================================

// Star Frame uses first 8 bytes as discriminator (derived from instruction name)
// Generated using: cargo run --manifest-path examples/Cargo.toml --bin get_discriminators
const INITIALIZE_LOTTERY_DISCRIMINATOR: [u8; 8] = [0x78, 0x8d, 0xf8, 0x2a, 0x5e, 0x1b, 0x22, 0x86];

const OPEN_NEXT_ROUND_DISCRIMINATOR: [u8; 8] = [0x18, 0x36, 0x20, 0x4d, 0x74, 0xc9, 0x0f, 0xc6];

// ============================================================================
// Instruction Args (must match program structs)
// ============================================================================

#[derive(BorshSerialize, Clone, Debug)]
struct InitializeLotteryArgs {
    ticket_price: u64,     // Price per ticket in USDC lamports
    default_duration: i64, // Default round duration in seconds
    fee_bps: u16,          // Fee in basis points (e.g., 500 = 5%)
}

#[derive(BorshSerialize, Clone, Debug)]
struct OpenNextRoundArgs {
    duration_seconds: i64, // How long the round should run
    round_id: u64,         // Expected round ID (must match current_round_id + 1)
}

// ============================================================================
// PDA Derivation Helpers
// ============================================================================

fn find_lottery_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"LOTTERY_CONFIG"], program_id)
}

fn find_treasury_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"TREASURY"], program_id)
}

fn find_round_pda(round_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"ROUND", &round_id.to_le_bytes()], program_id)
}

fn find_round_vault_pda(round_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"ROUND_VAULT", &round_id.to_le_bytes()], program_id)
}

// ============================================================================
// Instruction Builders
// ============================================================================

/// Build InitializeLottery instruction
fn build_initialize_lottery_ix(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    ticket_price: u64,
    default_duration: i64,
    fee_bps: u16,
) -> Instruction {
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    let (lottery_config, _) = find_lottery_config_pda(&program_id);
    let (treasury_pda, _) = find_treasury_pda(&program_id);

    // Derive ATA manually using seeds

    let treasury_ata = get_associated_token_address(&treasury_pda, usdc_mint);

    let args = InitializeLotteryArgs {
        ticket_price,
        default_duration,
        fee_bps,
    };

    // Serialize: discriminator + args
    let mut data = INITIALIZE_LOTTERY_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&borsh_to_vec(&args).unwrap());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*admin, true),               // admin (signer, funder)
            AccountMeta::new(lottery_config, false),      // lottery_config
            AccountMeta::new_readonly(*usdc_mint, false), // usdc_mint
            AccountMeta::new(treasury_pda, false),        // treasury_pda
            AccountMeta::new(treasury_ata, false),        // treasury_ata
            AccountMeta::new_readonly(system_program_id(), false), // system_program
            AccountMeta::new_readonly(token_program_id, false), // token_program
            AccountMeta::new_readonly(associated_token_program_id, false), // associated_token_program
        ],
        data,
    }
}

/// Build OpenNextRound instruction
fn build_open_next_round_ix(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    round_id: u64,
    duration_seconds: i64,
) -> Instruction {
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    let (lottery_config, _) = find_lottery_config_pda(&program_id);
    let (round, _) = find_round_pda(round_id, &program_id);
    let (round_vault, _) = find_round_vault_pda(round_id, &program_id);

    let args = OpenNextRoundArgs {
        duration_seconds,
        round_id,
    };

    // Serialize: discriminator + args
    let mut data = OPEN_NEXT_ROUND_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&borsh_to_vec(&args).unwrap());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*admin, true),          // admin (signer, funder)
            AccountMeta::new(lottery_config, false), // lottery_config
            AccountMeta::new(round, false),          // round
            AccountMeta::new_readonly(round_vault, false), // round_vault
            AccountMeta::new_readonly(*usdc_mint, false), // usdc_mint
            AccountMeta::new_readonly(system_program_id(), false), // system_program
            AccountMeta::new_readonly(token_program_id, false), // token_program
        ],
        data,
    }
}

// ============================================================================
// Client Operations
// ============================================================================

struct LotteryClient {
    rpc_client: RpcClient,
    payer: Keypair,
    program_id: Pubkey,
}

impl LotteryClient {
    /// Create a new lottery client
    pub fn new(rpc_url: &str, payer: Keypair) -> Self {
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

        Self {
            rpc_client,
            payer,
            program_id,
        }
    }

    /// Initialize the lottery program
    pub fn initialize_lottery(
        &self,
        usdc_mint: &Pubkey,
        ticket_price: u64,
        default_duration: i64,
        fee_bps: u16,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let admin = self.payer.pubkey();

        let ix =
            build_initialize_lottery_ix(&admin, usdc_mint, ticket_price, default_duration, fee_bps);

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&tx)?;

        println!("Lottery initialized!");
        println!("   Signature: {}", signature);
        let (config_pda, _) = find_lottery_config_pda(&self.program_id);
        println!("   Config PDA: {}", config_pda);
        let (treasury_pda, _) = find_treasury_pda(&self.program_id);
        println!("   Treasury PDA: {}", treasury_pda);
        // Derive ATA manually
        let seeds = &[
            treasury_pda.as_ref(),
            token_program_id.as_ref(),
            usdc_mint.as_ref(),
        ];
        let (treasury_ata, _) = Pubkey::find_program_address(seeds, &associated_token_program_id);
        println!("   Treasury ATA: {}", treasury_ata);

        Ok(signature.to_string())
    }

    /// Open a new lottery round
    pub fn open_next_round(
        &self,
        usdc_mint: &Pubkey,
        round_id: u64,
        duration_seconds: i64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let admin = self.payer.pubkey();

        let ix = build_open_next_round_ix(&admin, usdc_mint, round_id, duration_seconds);

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&tx)?;

        println!(" Round {} opened!", round_id);
        println!("   Signature: {}", signature);
        let (round_pda, _) = find_round_pda(round_id, &self.program_id);
        println!("   Round PDA: {}", round_pda);
        let (vault_pda, _) = find_round_vault_pda(round_id, &self.program_id);
        println!("   Round Vault: {}", vault_pda);
        println!("   Duration: {} seconds", duration_seconds);

        Ok(signature.to_string())
    }

    /// Get lottery config account
    pub fn get_config(&self) -> (Pubkey, u8) {
        find_lottery_config_pda(&self.program_id)
    }

    /// Get round PDA for a specific round ID
    pub fn get_round(&self, round_id: u64) -> (Pubkey, u8) {
        find_round_pda(round_id, &self.program_id)
    }
}

// ============================================================================
// Example Usage
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("<� Solana Lottery Pool - Client Example\n");

    // Configuration
    let rpc_url = "https://api.devnet.solana.com";
    let usdc_mint = Pubkey::from_str(USDC_MINT_DEVNET)?;

    // Load payer keypair (you'll need to provide your own)
    // For this example, we'll generate a new one
    let payer = Keypair::new();
    println!("=d Admin/Payer: {}", payer.pubkey());
    println!("�  You'll need to fund this account with devnet SOL\n");

    // Create client
    let client = LotteryClient::new(rpc_url, payer);

    // Example 1: Initialize Lottery
    println!("=� Step 1: Initialize Lottery");
    println!("   Ticket Price: 1 USDC (1_000_000 lamports)");
    println!("   Default Duration: 1 hour (3600 seconds)");
    println!("   Fee: 5% (500 bps)\n");

    // Uncomment to run:
    // client.initialize_lottery(
    //     &usdc_mint,
    //     1_000_000,  // 1 USDC
    //     3600,       // 1 hour
    //     500,        // 5%
    // )?;

    // Example 2: Open First Round
    println!("\n=� Step 2: Open First Round");
    println!("   Round ID: 1");
    println!("   Duration: 24 hours (86400 seconds)\n");

    // Print account addresses for reference
    println!("\n=� Account Reference:");
    let (config_pda, _) = client.get_config();
    println!("   Lottery Config: {}", config_pda);
    let (round1_pda, _) = client.get_round(1);
    println!("   Round 1 PDA: {}", round1_pda);
    let (treasury_pda, _) = find_treasury_pda(&client.program_id);

    // Derive ATA manually
    let seeds = &[
        treasury_pda.as_ref(),
        token_program_id.as_ref(),
        usdc_mint.as_ref(),
    ];
    let (treasury_ata, _) = Pubkey::find_program_address(seeds, &associated_token_program_id);
    println!("   Treasury ATA: {}", treasury_ata);

    println!("\n( Done! Uncomment the instruction calls to run them.");

    Ok(())
}
