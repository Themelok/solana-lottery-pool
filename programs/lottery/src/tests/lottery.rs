use crate::states::*;
use crate::*;
use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use mollusk_svm_programs_token::token;
use solana_account::Account as SolanaAccount;
use solana_program_option::COption;
use spl_token_interface::state::{Account as TokenAccountData, AccountState, Mint};
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
    let mollusk = Mollusk::new(&SolanaLotteryPoolProgram::ID, &program_path());

    // Setup accounts
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();

    // Derive PDAs
    let config_seeds = LotteryConfigSeeds;
    let (lottery_config, _config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (treasury, _treasury_bump) =
        Pubkey::find_program_address(&treasury_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create proper USDC mint account (6 decimals like USDC)
    let mint_data = Mint {
        mint_authority: COption::Some(mint_authority),
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::<Pubkey>::None,
    };
    let usdc_mint_account = token::create_account_for_mint(mint_data);

    // Create treasury token account for USDC
    let treasury_account_data = TokenAccountData {
        mint: usdc_mint,
        owner: treasury,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let treasury_token_account = token::create_account_for_token_account(treasury_account_data);

    // Create mollusk context with accounts
    let mollusk = mollusk.with_context(HashMap::from_iter([
        (admin, SolanaAccount::new(1_000_000_000, 0, &System::ID)),
        (lottery_config, SolanaAccount::new(0, 0, &System::ID)),
        (treasury, treasury_token_account),
        (usdc_mint, usdc_mint_account),
        keyed_account_for_system_program(),
        token::keyed_account(),
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
                usdc_mint,
                treasury,
                system_program: None,
                token_program: None,
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
    let mint_authority = Pubkey::new_unique();

    // Derive PDAs
    let config_seeds = LotteryConfigSeeds;
    let (lottery_config, _config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (treasury, _treasury_bump) =
        Pubkey::find_program_address(&treasury_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create proper USDC mint account (6 decimals like USDC)
    let mint_data = Mint {
        mint_authority: COption::Some(mint_authority),
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::<Pubkey>::None,
    };
    let usdc_mint_account = token::create_account_for_mint(mint_data);

    // Create treasury token account for USDC
    let treasury_account_data = TokenAccountData {
        mint: usdc_mint,
        owner: treasury,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized, // Initialized
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let treasury_token_account = token::create_account_for_token_account(treasury_account_data);

    // Create mollusk context with accounts
    let mollusk = mollusk.with_context(HashMap::from_iter([
        (admin, SolanaAccount::new(1_000_000_000, 0, &System::ID)),
        (lottery_config, SolanaAccount::new(0, 0, &System::ID)),
        (treasury, treasury_token_account),
        (usdc_mint, usdc_mint_account),
        keyed_account_for_system_program(),
        token::keyed_account(),
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
            usdc_mint,
            treasury,
            system_program: None,
            token_program: None,
        },
    )?);

    // Verify it failed with the expected error
    assert!(
        result.program_result.is_err(),
        "Expected initialization to fail with invalid fee"
    );

    Ok(())
}

#[test]
fn test_open_next_round() -> Result<(), Box<dyn Error>> {
    let mollusk = Mollusk::new(&SolanaLotteryPoolProgram::ID, &program_path());

    // Setup accounts
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();

    // Derive PDAs
    let config_seeds = LotteryConfigSeeds;
    let (lottery_config, config_bump) =
        Pubkey::find_program_address(&config_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let treasury_seeds = TreasurySeeds;
    let (treasury, treasury_bump) =
        Pubkey::find_program_address(&treasury_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create proper USDC mint account
    let mint_data = Mint {
        mint_authority: COption::Some(mint_authority),
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::<Pubkey>::None,
    };
    let usdc_mint_account = token::create_account_for_mint(mint_data);

    // Create treasury token account
    let treasury_account_data = TokenAccountData {
        mint: usdc_mint,
        owner: treasury,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let treasury_token_account = token::create_account_for_token_account(treasury_account_data);

    // Derive round accounts upfront
    let round_id = 1u64;
    let round_seeds = RoundSeeds { round_id };
    let (round_pda, round_bump) =
        Pubkey::find_program_address(&round_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    let vault_seeds = RoundVaultSeeds { round_id };
    let (round_vault_pda, vault_bump) =
        Pubkey::find_program_address(&vault_seeds.seeds(), &SolanaLotteryPoolProgram::ID);

    // Create mollusk context with accounts
    let mollusk = mollusk.with_context(HashMap::from_iter([
        (admin, SolanaAccount::new(1_000_000_000, 0, &System::ID)),
        (lottery_config, SolanaAccount::new(0, 0, &System::ID)),
        (treasury, treasury_token_account),
        (usdc_mint, usdc_mint_account),
        (round_pda, SolanaAccount::new(0, 0, &System::ID)),
        (round_vault_pda, SolanaAccount::new(0, 0, &System::ID)),
        keyed_account_for_system_program(),
        token::keyed_account(),
    ]));

    // Step 1: Initialize lottery first
    let ticket_price = 1_000_000u64; // 1 USDC
    let default_duration = 300i64; // 5 minutes
    let fee_bps = 500u16; // 5%

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
                usdc_mint,
                treasury,
                system_program: None,
                token_program: None,
            },
        )?,
        &[Check::success()],
    );

    // Step 2: Open next round
    let duration_seconds = 600i64; // 10 minutes

    let result = mollusk.process_instruction(&SolanaLotteryPoolProgram::instruction(
        &OpenNextRound {
            args: OpenNextRoundArgs {
                duration_seconds,
                round_id,
            },
        },
        OpenNextRoundClientAccounts {
            admin,
            lottery_config,
            round: round_pda,
            round_vault: round_vault_pda,
            usdc_mint,
            system_program: None,
            token_program: None,
        },
    )?);

    // Verify success
    assert!(
        result.program_result.is_ok(),
        "Open next round should succeed: {:?}",
        result.program_result
    );

    // Verify lottery_config was updated
    let config_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| pubkey == &lottery_config)
        .expect("Config account should exist");

    let config_data: LotteryConfig = LotteryConfig::deserialize_account(&config_account.1.data)?;
    let current_round_id = config_data.current_round_id;
    assert_eq!(
        current_round_id, 1,
        "Round ID should be incremented to 1"
    );

    // Verify round state
    let round_account = result
        .resulting_accounts
        .iter()
        .find(|(pubkey, _)| pubkey == &round_pda)
        .expect("Round account should exist");

    let round_data: Round = Round::deserialize_account(&round_account.1.data)?;

    // Copy packed fields to avoid alignment issues
    let id = round_data.id;
    let round_ticket_price = round_data.ticket_price;
    let total_tickets = round_data.total_tickets;
    let pot_amount = round_data.pot_amount;
    let start_time = round_data.start_time;
    let end_time = round_data.end_time;
    let round_bump_actual = round_data.bump;
    let vault_bump_actual = round_data.vault_bump;

    assert_eq!(id, 1, "Round ID should be 1");
    assert_eq!(
        round_ticket_price, ticket_price,
        "Ticket price should match config"
    );
    assert_eq!(
        total_tickets, 0,
        "Total tickets should start at 0"
    );
    assert_eq!(pot_amount, 0, "Pot amount should start at 0");
    assert_eq!(
        round_data.status(),
        RoundStatus::Open,
        "Round should be Open"
    );
    assert_eq!(round_bump_actual, round_bump, "Round bump should match");
    assert_eq!(
        vault_bump_actual, vault_bump,
        "Vault bump should match"
    );
    assert!(
        end_time > start_time,
        "End time should be after start time"
    );
    assert_eq!(
        end_time - start_time,
        duration_seconds,
        "Duration should match"
    );

    Ok(())
}
