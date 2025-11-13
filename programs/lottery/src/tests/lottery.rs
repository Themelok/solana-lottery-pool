use crate::states::*;
use crate::*;
use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use solana_account::Account as SolanaAccount;
use star_frame::{client::MakeInstruction, prelude::Pubkey, program::StarFrameProgram};
use std::collections::HashMap;
use std::env;
use std::error::Error;

fn program_path() -> String {
    env::var("CARGO_MANIFEST_DIR")
        .map(|dir| format!("{}/../../target/deploy/solana_lottery_pool", dir))
        .unwrap_or_else(|_| "../../target/deploy/solana_lottery_pool".to_string())
}

#[test]
fn test_initialize_lottery() -> Result<(), Box<dyn Error>> {
    // Use absolute path to .so file

    let mollusk = Mollusk::new(&SolanaLotteryPoolProgram::ID, &program_path());

    // Setup accounts
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    // Derive PDAs
    let config_seeds = LotteryConfigSeeds;
    let (lottery_config, _config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (treasury, _treasury_bump) =
        Pubkey::find_program_address(&treasury_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create mollusk context with accounts
    let mollusk = mollusk.with_context(HashMap::from_iter([
        (admin, SolanaAccount::new(1_000_000_000, 0, &System::ID)),
        (lottery_config, SolanaAccount::new(0, 0, &System::ID)),
        (treasury, SolanaAccount::new(0, 0, &System::ID)),
        (usdc_mint, SolanaAccount::new(0, 0, &System::ID)),
        keyed_account_for_system_program(),
    ]));

    // Instruction args
    let ticket_price = 1_000_000u64; // 1 USDC
    let default_duration = 300i64; // 5 minutes
    let fee_bps = 500u16; // 5%

    // Expected state after initialization
    let expected_config = LotteryConfig {
        admin,
        treasury,
        usdc_mint,
        ticket_price,
        default_duration,
        fee_bps,
        current_round_id: 0,
        bump: _config_bump,
        treasury_bump: _treasury_bump,
        _padding: [0; 6],
    };

    // Initialize lottery
    mollusk.process_and_validate_instruction(
        &SolanaLotteryPoolProgram::instruction(
            &InitializeLottery {
                args: InitializeLotteryArgs {
                    ticket_price,
                    default_duration,
                    fee_bps,
                },
            },
            InitializeLotteryClientAccounts {
                admin,
                lottery_config,
                treasury,
                usdc_mint,
                system_program: None,
            },
        )?,
        &[
            Check::success(),
            Check::account(&lottery_config)
                .data(&LotteryConfig::serialize_account(expected_config)?)
                .owner(&SolanaLotteryPoolProgram::ID)
                .build(),
        ],
    );

    Ok(())
}

#[test]
fn test_initialize_lottery_with_invalid_fee() -> Result<(), Box<dyn Error>> {
    let mollusk = Mollusk::new(&SolanaLotteryPoolProgram::ID, &program_path());

    // Setup accounts
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    // Derive PDAs
    let config_seeds = LotteryConfigSeeds;
    let (lottery_config, _config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (treasury, _treasury_bump) =
        Pubkey::find_program_address(&treasury_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create mollusk context with accounts
    let mollusk = mollusk.with_context(HashMap::from_iter([
        (admin, SolanaAccount::new(1_000_000_000, 0, &System::ID)),
        (lottery_config, SolanaAccount::new(0, 0, &System::ID)),
        (treasury, SolanaAccount::new(0, 0, &System::ID)),
        (usdc_mint, SolanaAccount::new(0, 0, &System::ID)),
        keyed_account_for_system_program(),
    ]));

    // Instruction args with INVALID fee (>= 10000 bps = 100%)
    let ticket_price = 1_000_000u64;
    let default_duration = 300i64;
    let fee_bps = 10_000u16; // Invalid: 100% fee

    // Try to initialize lottery - should fail
    let result = mollusk.process_instruction(&SolanaLotteryPoolProgram::instruction(
        &InitializeLottery {
            args: InitializeLotteryArgs {
                ticket_price,
                default_duration,
                fee_bps,
            },
        },
        InitializeLotteryClientAccounts {
            admin,
            lottery_config,
            treasury,
            usdc_mint,
            system_program: None,
        },
    )?);

    // Verify it failed with the expected error
    assert!(
        result.program_result.is_err(),
        "Expected initialization to fail with invalid fee"
    );

    Ok(())
}
