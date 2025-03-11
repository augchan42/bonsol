use {
    bonsol_interface::callback::{handle_callback, BonsolCallback},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        system_program::ID as SYSTEM_PROGRAM_ID,
        sysvar::Sysvar,
    },
    thiserror::Error,
};

// The expected image ID for our 8BitOracle I Ching program
pub const BITORACLE_ICHING_IMAGE_ID: &str = "68f4b0c5f9ce034aa60ceb264a18d6c410a3af68fafd931bcfd9ebe7c1e42960";

// Seeds for PDA derivation
pub const HEXAGRAM_SEED_PREFIX: &[u8] = b"8bitoracle-hexagram";
pub const HEXAGRAM_SEED_VERSION: &[u8] = b"v1";

#[derive(Error, Debug)]
pub enum CallbackError {
    #[error("Invalid instruction data")]
    InvalidInstruction,
    #[error("Account not rent exempt")]
    NotRentExempt,
    #[error("Invalid hexagram data")]
    InvalidHexagramData,
    #[error("Invalid PDA")]
    InvalidPDA,
    #[error("Invalid signer")]
    InvalidSigner,
    #[error("Invalid account owner")]
    InvalidOwner,
    #[error("Invalid system program")]
    InvalidSystemProgram,
}

impl From<CallbackError> for ProgramError {
    fn from(e: CallbackError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// Structure to store hexagram data on-chain
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct HexagramData {
    pub lines: [u8; 6],         // The 6,7,8,9 values for each line
    pub ascii_art: String,      // The ASCII representation
    pub timestamp: i64,         // When the reading was done
    pub is_initialized: bool,    // To check if the account is initialized
}

/// Derives the hexagram storage PDA for a given execution account
pub fn derive_hexagram_address(execution_account: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            HEXAGRAM_SEED_PREFIX,
            HEXAGRAM_SEED_VERSION,
            execution_account.as_ref(),
        ],
        program_id,
    )
}

/// Process a single output from the callback data
fn process_output(output: &[u8], lines: &mut [u8; 6]) -> Option<String> {
    if output.starts_with(&[b'-', b'-', b'-']) {
        // This is the ASCII art section
        let ascii_art = String::from_utf8_lossy(output).to_string();
        msg!("Found ASCII art output ({} bytes)", output.len());
        Some(ascii_art)
    } else if output.len() == 6 {
        // This is the lines data section
        lines.copy_from_slice(output);
        msg!("Found lines data output: {:?}", output);
        None
    } else {
        None
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("8BitOracle I Ching Callback - Processing instruction");
    
    let accounts_iter = &mut accounts.iter();
    
    // First account is the execution account (signer)
    let execution_account = next_account_info(accounts_iter)?;
    // Second account is where we'll store the hexagram data
    let hexagram_account = next_account_info(accounts_iter)?;
    // Get the system program account
    let system_program = next_account_info(accounts_iter)?;
    
    // Verify account permissions and ownership
    if !execution_account.is_signer {
        msg!("Error: Execution account must be a signer");
        return Err(CallbackError::InvalidSigner.into());
    }

    // Verify system program
    if system_program.key != &SYSTEM_PROGRAM_ID {
        msg!("Error: Invalid system program");
        return Err(CallbackError::InvalidSystemProgram.into());
    }

    // Verify the hexagram account PDA
    let (expected_hexagram_address, bump_seed) = derive_hexagram_address(execution_account.key, program_id);
    if expected_hexagram_address != *hexagram_account.key {
        msg!("Error: Hexagram account does not match PDA derivation");
        msg!("Expected: {}", expected_hexagram_address);
        msg!("Received: {}", hexagram_account.key);
        return Err(CallbackError::InvalidPDA.into());
    }
    
    // Parse the callback data using the helper
    let callback_data: BonsolCallback = handle_callback(
        BITORACLE_ICHING_IMAGE_ID,
        execution_account.key,
        accounts,
        instruction_data,
    )?;
    
    // Add detailed diagnostic logging
    msg!(
        "Processing callback:\n\
         - Execution account: {}\n\
         - Storage account: {}\n\
         - Input digest size: {}\n\
         - Number of outputs: {}",
        execution_account.key,
        hexagram_account.key,
        callback_data.input_digest.len(),
        callback_data.committed_outputs.len()
    );
    
    // Parse the committed outputs
    let mut lines = [0u8; 6];
    let mut ascii_art = String::new();
    
    // Process each output
    if let Some(art) = process_output(&callback_data.committed_outputs, &mut lines) {
        ascii_art = art;
    }
    
    // Validate the data
    if ascii_art.is_empty() || lines.iter().all(|&x| x == 0) {
        msg!("Invalid hexagram data received");
        msg!("ASCII art empty: {}", ascii_art.is_empty());
        msg!("Lines all zero: {}", lines.iter().all(|&x| x == 0));
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Create the hexagram data account if it doesn't exist
    if hexagram_account.data_is_empty() {
        msg!("Creating new 8BitOracle hexagram account");
        let rent = Rent::get()?;
        let space = 1000; // Enough space for the struct with ASCII art
        let lamports = rent.minimum_balance(space);
        
        msg!(
            "Account creation details:\n\
             - Space: {} bytes\n\
             - Lamports: {}\n\
             - Rent exempt minimum: {}",
            space,
            lamports,
            rent.minimum_balance(space)
        );
        
        // Create with PDA
        let seeds = &[
            HEXAGRAM_SEED_PREFIX,
            HEXAGRAM_SEED_VERSION,
            execution_account.key.as_ref(),
            &[bump_seed],
        ];
        
        solana_program::program::invoke_signed(
            &system_instruction::create_account(
                execution_account.key,
                &expected_hexagram_address,
                lamports,
                space as u64,
                program_id,
            ),
            &[
                execution_account.clone(),
                hexagram_account.clone(),
                system_program.clone(),
            ],
            &[seeds],
        )?;
        
        msg!("Hexagram account created successfully");
    }
    
    // Create and serialize the hexagram data
    let hexagram = HexagramData {
        lines,
        ascii_art: ascii_art.clone(),
        timestamp: solana_program::clock::Clock::get()?.unix_timestamp,
        is_initialized: true,
    };
    
    // Store the data
    hexagram.serialize(&mut *hexagram_account.try_borrow_mut_data()?)?;
    
    msg!("8BitOracle hexagram data stored successfully");
    msg!(
        "Final hexagram state:\n\
         - Lines: {:?}\n\
         - Timestamp: {}\n\
         - ASCII Art:\n{}",
        lines,
        hexagram.timestamp,
        ascii_art
    );
    
    Ok(())
} 