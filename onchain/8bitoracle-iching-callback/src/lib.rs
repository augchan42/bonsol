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
pub const CALLBACK_VERSION: &str = "v0.1.4"; // Increment this each time we deploy

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
    msg!("Instruction data length: {} bytes", instruction_data.len());
    msg!("First 32 bytes of instruction data: {:02x?}", &instruction_data[..32.min(instruction_data.len())]);
    
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
    msg!("  - Input digest: {:02x?}", callback_data.input_digest);
    msg!("  - First 32 bytes of outputs: {:02x?}", &callback_data.committed_outputs[..32.min(callback_data.committed_outputs.len())]);
    
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
        msg!("  - Full output: {:02x?}", outputs);
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
        msg!("Expected: {:02x?}", callback_data.input_digest);
        msg!("Got:      {:02x?}", &outputs[..32]);
        msg!("Full output: {:02x?}", outputs);
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
        msg!("Expected: 0xaa");
        msg!("Got:      {:#04x}", outputs[32]);
        msg!("This indicates the output format is incorrect");
        msg!("Full output: {:02x?}", outputs);
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
    msg!("Checking each line value:");
    for (i, &line) in lines.iter().enumerate() {
        msg!("Line {}: {} (valid: {})", i + 1, line, (6..=9).contains(&line));
    }
    let valid_lines = lines.iter().all(|&x| (6..=9).contains(&x));
    if !valid_lines {
        msg!("❌ Line values validation failed!");
        msg!("Invalid line values detected. Each value must be between 6 and 9.");
        msg!("Line values (hex): {:02x?}", lines);
        msg!("Line values (dec): {:?}", lines);
        msg!("Invalid values: {:?}", lines.iter().enumerate().filter(|(_, &x)| !(6..=9).contains(&x)).collect::<Vec<_>>());
        msg!("Full output: {:02x?}", outputs);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Line values validation passed");
    
    // Remaining bytes are ASCII art
    msg!("\n🔍 ASCII Art Validation");
    let ascii_art = String::from_utf8_lossy(&outputs[39..]).to_string();
    msg!("Expected length: 47 bytes");
    msg!("Actual length: {} bytes", outputs.len() - 39);
    msg!("ASCII art bytes: {:02x?}", &outputs[39..]);
    msg!("ASCII art content:\n{}", ascii_art);
    msg!("ASCII art byte values:");
    for (i, &byte) in outputs[39..].iter().enumerate() {
        msg!("Byte {}: {:#04x} ({})", i, byte, byte as char);
    }
    
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
            msg!("ASCII art bytes: {:02x?}", &outputs[39..]);
        }
        if lines.iter().all(|&x| x == 0) {
            msg!("Error: All line values are zero");
            msg!("Line values: {:?}", lines);
        }
        msg!("Full output: {:02x?}", outputs);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    msg!("✓ Final validation passed");
    
    // Create the hexagram data account if it doesn't exist
    if hexagram_account.data_is_empty() {
        msg!("\n🔍 Account Creation");
        msg!("Initial account state:");
        msg!("  - Data length: {} bytes", hexagram_account.data_len());
        msg!("  - Lamports: {}", hexagram_account.lamports());
        msg!("  - Owner: {}", hexagram_account.owner);
        msg!("  - Executable: {}", hexagram_account.executable);
        msg!("  - Rent epoch: {}", hexagram_account.rent_epoch);

        let rent = Rent::get()?;
        let space = 8 + 6 + 1024 + 8 + 1;
        let lamports = rent.minimum_balance(space);
        
        msg!(
            "Account creation parameters:\n\
             - Space: {} bytes\n\
             - Breakdown:\n\
               • Discriminator: 8 bytes\n\
               • Lines array: 6 bytes\n\
               • ASCII art: 1024 bytes\n\
               • Timestamp: 8 bytes\n\
               • Initialized flag: 1 byte\n\
             - Lamports needed: {}\n\
             - Rent exempt minimum: {}\n\
             - Payer: {}\n\
             - New account: {}\n\
             - Owner (program): {}",
            space,
            lamports,
            rent.minimum_balance(space),
            execution_account.key,
            hexagram_account.key,
            program_id
        );
        
        msg!("Creating account with system instruction...");
        let create_account_ix = system_instruction::create_account(
            execution_account.key,
            hexagram_account.key,
            lamports,
            space as u64,
            program_id,
        );
        msg!("System instruction created:");
        msg!("  - Program: {}", create_account_ix.program_id);
        msg!("  - Accounts: {} accounts", create_account_ix.accounts.len());
        for (i, acc) in create_account_ix.accounts.iter().enumerate() {
            msg!("    {}. pubkey: {}, is_signer: {}, is_writable: {}", 
                i + 1, acc.pubkey, acc.is_signer, acc.is_writable);
        }
        
        msg!("Invoking system instruction...");
        solana_program::program::invoke(
            &create_account_ix,
            &[
                execution_account.clone(),
                hexagram_account.clone(),
                system_program.clone(),
            ],
        )?;
        
        msg!("\n🔍 Account Creation Result");
        msg!("Account state after creation:");
        msg!("  - Data length: {} bytes", hexagram_account.data_len());
        msg!("  - Lamports: {}", hexagram_account.lamports());
        msg!("  - Owner: {}", hexagram_account.owner);
        msg!("  - Executable: {}", hexagram_account.executable);
        msg!("  - Rent epoch: {}", hexagram_account.rent_epoch);
        
        if hexagram_account.data_len() != space {
            msg!("❌ WARNING: Account size mismatch!");
            msg!("  - Expected: {} bytes", space);
            msg!("  - Actual: {} bytes", hexagram_account.data_len());
        } else {
            msg!("✓ Account created with correct size");
        }
    }
    
    msg!("\n🔍 Data Serialization");
    msg!("Preparing hexagram data:");
    msg!("  - Lines array size: {} bytes", std::mem::size_of::<[u8; 6]>());
    msg!("  - ASCII art length: {} bytes", ascii_art.len());
    msg!("  - Timestamp size: {} bytes", std::mem::size_of::<i64>());
    msg!("  - Bool size: {} byte", std::mem::size_of::<bool>());
    
    // Create and serialize the hexagram data
    let hexagram = HexagramData {
        lines,
        ascii_art: ascii_art.clone(),
        timestamp: solana_program::clock::Clock::get()?.unix_timestamp,
        is_initialized: true,
    };
    
    msg!("Serializing data...");
    let mut data = hexagram_account.try_borrow_mut_data()?;
    msg!("  - Available buffer size: {} bytes", data.len());
    msg!("  - Required size: {} bytes", 8 + 6 + ascii_art.len() + 8 + 1);
    
    // Store the data
    hexagram.serialize(&mut *data)?;
    
    msg!("\n🔍 Final Account State");
    msg!("Account data after serialization:");
    msg!("  - Data length: {} bytes", hexagram_account.data_len());
    msg!("  - First 32 bytes: {:02x?}", &hexagram_account.try_borrow_data()?[..32.min(hexagram_account.data_len())]);
    msg!("  - Owner: {}", hexagram_account.owner);
    msg!("  - Lamports: {}", hexagram_account.lamports());
    
    msg!("8BitOracle hexagram data stored successfully");
    msg!(
        "Final hexagram state:\n\
         - Lines: {:?}\n\
         - ASCII art length: {} bytes\n\
         - ASCII art content:\n{}\n\
         - Timestamp: {}\n\
         - Is initialized: true",
        lines,
        ascii_art.len(),
        ascii_art,
        hexagram.timestamp
    );
    
    Ok(())
} 