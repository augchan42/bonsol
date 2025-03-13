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
pub const BITORACLE_ICHING_IMAGE_ID: &str = "5a883ffc803d906106d4f6512a17dfd8279d8f03197e120b9f0ac54673b8544b";

// Add version constant
pub const CALLBACK_VERSION: &str = "v0.1.3"; // Increment this each time we deploy

#[derive(Error, Debug)]
pub enum CallbackError {
    #[error("Invalid instruction data")]
    InvalidInstruction,
    #[error("Account not rent exempt")]
    NotRentExempt,
    #[error("Invalid hexagram data")]
    InvalidHexagramData,
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

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("================================================================");
    msg!("8BitOracle I Ching Callback {} - Starting instruction", CALLBACK_VERSION);
    msg!("================================================================");
    
    // Get accounts in order
    let accounts_iter = &mut accounts.iter();
    
    msg!("🔍 Validating account list...");
    msg!("Expected accounts:");
    msg!("1. Execution account (readonly signer)");
    msg!("2. Hexagram storage account (writable)");
    msg!("3. System program (readonly, executable)");
    msg!("Actual accounts provided: {}", accounts.len());
    
    // First account is always the execution account (readonly signer)
    let execution_account = next_account_info(accounts_iter)?;
    msg!("\n📋 Execution Account Validation");
    msg!("Expected:");
    msg!("  - Key: Any valid pubkey");
    msg!("  - Signer: true");
    msg!("  - Writable: false");
    msg!("  - Owner: Any valid program");
    msg!("Actual:");
    msg!("  - Key: {}", execution_account.key);
    msg!("  - Signer: {}", execution_account.is_signer);
    msg!("  - Writable: {}", execution_account.is_writable);
    msg!("  - Owner: {}", execution_account.owner);
    
    if !execution_account.is_signer {
        msg!("❌ Validation failed: Execution account must be a signer");
        return Err(CallbackError::InvalidSigner.into());
    }
    if execution_account.is_writable {
        msg!("❌ Validation failed: Execution account must be readonly");
        return Err(CallbackError::InvalidInstruction.into());
    }
    msg!("✓ Execution account validation passed");
    
    // Next get the hexagram storage account
    let hexagram_account = next_account_info(accounts_iter)?;
    msg!("\n📋 Hexagram Storage Account Validation");
    msg!("Expected:");
    msg!("  - Key: Any valid pubkey");
    msg!("  - Signer: false");
    msg!("  - Writable: true");
    msg!("  - Owner: Either System Program (if new) or this program (if existing)");
    msg!("Actual:");
    msg!("  - Key: {}", hexagram_account.key);
    msg!("  - Signer: {}", hexagram_account.is_signer);
    msg!("  - Writable: {}", hexagram_account.is_writable);
    msg!("  - Owner: {}", hexagram_account.owner);
    msg!("  - Data length: {} bytes", hexagram_account.data_len());
    msg!("  - Lamports: {}", hexagram_account.lamports());
    
    if !hexagram_account.is_writable {
        msg!("❌ Validation failed: Hexagram account must be writable");
        return Err(CallbackError::InvalidInstruction.into());
    }
    if hexagram_account.is_signer {
        msg!("❌ Validation failed: Hexagram account must not be a signer");
        return Err(CallbackError::InvalidSigner.into());
    }
    msg!("✓ Hexagram account validation passed");
    
    // Finally get the system program
    let system_program = next_account_info(accounts_iter)?;
    msg!("\n📋 System Program Validation");
    msg!("Expected:");
    msg!("  - Key: {}", SYSTEM_PROGRAM_ID);
    msg!("  - Signer: false");
    msg!("  - Writable: false");
    msg!("  - Executable: true");
    msg!("Actual:");
    msg!("  - Key: {}", system_program.key);
    msg!("  - Signer: {}", system_program.is_signer);
    msg!("  - Writable: {}", system_program.is_writable);
    msg!("  - Executable: {}", system_program.executable);
    
    // Verify system program
    if !system_program.executable || system_program.key != &SYSTEM_PROGRAM_ID {
        msg!("❌ Validation failed: Invalid system program");
        msg!("  - Expected key: {}", SYSTEM_PROGRAM_ID);
        msg!("  - Got key: {}", system_program.key);
        msg!("  - Expected executable: true");
        msg!("  - Got executable: {}", system_program.executable);
        return Err(CallbackError::InvalidSystemProgram.into());
    }
    msg!("✓ System program validation passed");

    msg!("\n🔍 Validating callback data...");
    msg!("Expected image ID: {}", BITORACLE_ICHING_IMAGE_ID);
    msg!("Expected execution account: {}", execution_account.key);
    
    // Parse the callback data using the helper
    let callback_data: BonsolCallback = handle_callback(
        BITORACLE_ICHING_IMAGE_ID,
        execution_account.key,
        accounts,
        instruction_data,
    )?;
    
    msg!("\n📋 Callback Data Validation");
    msg!("Expected structure:");
    msg!("  - Input digest: 32 bytes");
    msg!("  - Committed outputs: 86 bytes total");
    msg!("    • First 32 bytes: Input digest");
    msg!("    • Next 1 byte: Marker (0xaa)");
    msg!("    • Next 6 bytes: Line values");
    msg!("    • Remaining 47 bytes: ASCII art");
    msg!("Actual:");
    msg!("  - Input digest: {} bytes", callback_data.input_digest.len());
    msg!("  - Committed outputs: {} bytes", callback_data.committed_outputs.len());
    
    // Process the committed outputs as a single byte slice
    let outputs = &callback_data.committed_outputs;
    msg!("\n🔍 Output Size Validation");
    msg!("Expected: 86 bytes");
    msg!("Actual: {} bytes", outputs.len());
    
    // Validate total size matches expected format
    if outputs.len() != 86 {
        msg!("❌ Size validation failed!");
        msg!("  - Expected: 86 bytes");
        msg!("  - Got: {} bytes", outputs.len());
        msg!("  - Missing: {} bytes", 86 - outputs.len());
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Size validation passed");
    
    // First 32 bytes are the input digest
    msg!("\n🔍 Input Digest Validation");
    msg!("Expected digest: {:02x?}", callback_data.input_digest);
    msg!("Actual digest:   {:02x?}", &outputs[..32]);
    if &outputs[..32] != callback_data.input_digest {
        msg!("❌ Input digest validation failed!");
        msg!("Digests do not match. This could indicate:");
        msg!("1. Data corruption during transmission");
        msg!("2. Incorrect input processing");
        msg!("3. Malformed callback data");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Input digest validation passed");
    
    // Next byte is the marker
    msg!("\n🔍 Marker Validation");
    msg!("Expected marker: 0xaa");
    msg!("Actual marker:   {:#04x}", outputs[32]);
    if outputs[32] != 0xaa {
        msg!("❌ Marker validation failed!");
        msg!("Invalid marker byte at position 32");
        msg!("This indicates the output format is incorrect");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Marker validation passed");
    
    // Next 6 bytes are the line values
    msg!("\n🔍 Line Values Validation");
    let mut lines = [0u8; 6];
    lines.copy_from_slice(&outputs[33..39]);
    msg!("Line values (hex): {:02x?}", lines);
    msg!("Line values (dec): {:?}", lines);
    msg!("Valid range for each line: 6-9");
    let valid_lines = lines.iter().all(|&x| (6..=9).contains(&x));
    if !valid_lines {
        msg!("❌ Line values validation failed!");
        msg!("Invalid line values detected. Each value must be between 6 and 9.");
        msg!("This indicates incorrect I Ching calculation");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Line values validation passed");
    
    // Remaining bytes are ASCII art
    msg!("\n🔍 ASCII Art Validation");
    let ascii_art = String::from_utf8_lossy(&outputs[39..]).to_string();
    msg!("Expected length: 47 bytes");
    msg!("Actual length: {} bytes", outputs.len() - 39);
    msg!("ASCII art content:\n{}", ascii_art);
    
    // Validate the data
    msg!("\n🔍 Final Data Validation");
    msg!("Checking:");
    msg!("1. ASCII art is not empty");
    msg!("2. Line values are not all zero");
    msg!("Results:");
    msg!("- ASCII art empty: {}", ascii_art.is_empty());
    msg!("- Lines all zero: {}", lines.iter().all(|&x| x == 0));
    
    if ascii_art.is_empty() || lines.iter().all(|&x| x == 0) {
        msg!("❌ Final validation failed!");
        if ascii_art.is_empty() {
            msg!("Error: Empty ASCII art");
        }
        if lines.iter().all(|&x| x == 0) {
            msg!("Error: All line values are zero");
        }
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Final validation passed");
    
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
        
        // Create account directly without PDA
        solana_program::program::invoke(
            &system_instruction::create_account(
                execution_account.key,
                hexagram_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[
                execution_account.clone(),
                hexagram_account.clone(),
                system_program.clone(),
            ],
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