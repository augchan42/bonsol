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
pub const BITORACLE_ICHING_IMAGE_ID: &str = "83fd7b6a7011b7b842f9ddc83dc7c470a0d4fb71fb6c8dd3064387bac21fd8de";

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
    
    // Log program and transaction details
    msg!("🔍 Program Details:");
    msg!("Program ID: {}", program_id);
    msg!("Number of accounts: {}", accounts.len());
    msg!("Instruction data length: {} bytes", instruction_data.len());
    
    // Log instruction data details
    msg!("\n📝 Instruction Data Analysis:");
    msg!("Raw instruction data: {:?}", instruction_data);
    if instruction_data.is_empty() {
        msg!("⚠️ Warning: Empty instruction data");
    } else {
        msg!("First byte (prefix): {}", instruction_data[0]);
        if instruction_data.len() > 1 {
            msg!("Remaining data length: {} bytes", instruction_data.len() - 1);
        }
    }
    
    // Detailed account analysis
    msg!("\n🔍 Detailed Account Analysis:");
    for (i, account) in accounts.iter().enumerate() {
        msg!("\nAccount {} Details:", i);
        msg!("  Address: {}", account.key);
        msg!("  Owner: {}", account.owner);
        msg!("  Lamports: {}", account.lamports());
        msg!("  Data Length: {} bytes", account.data_len());
        msg!("  Executable: {}", account.executable);
        msg!("  Rent Epoch: {}", account.rent_epoch);
        msg!("  Is Signer: {}", account.is_signer);
        msg!("  Is Writable: {}", account.is_writable);
    }
    
    msg!("\n🔄 Processing Callback Data");
    msg!("Expected Image ID: {}", BITORACLE_ICHING_IMAGE_ID);
    
    // Strip and log instruction data
    let stripped_data = if instruction_data.len() > 1 {
        msg!("Stripping first byte from instruction data");
        msg!("Original length: {}, New length: {}", instruction_data.len(), instruction_data.len() - 1);
        &instruction_data[1..]
    } else {
        msg!("❌ Error: Instruction data too short");
        msg!("Expected length > 1, got {}", instruction_data.len());
        return Err(CallbackError::InvalidInstruction.into());
    };
    
    msg!("\n🔍 Attempting handle_callback");
    msg!("Using execution account at index 1: {}", accounts[1].key);
    
    // Process callback data
    let callback_data: BonsolCallback = match handle_callback(
        BITORACLE_ICHING_IMAGE_ID,
        &accounts[1].key,
        accounts,
        stripped_data,
    ) {
        Ok(data) => {
            msg!("✓ handle_callback successful");
            msg!("Input digest length: {} bytes", data.input_digest.len());
            msg!("Committed outputs length: {} bytes", data.committed_outputs.len());
            data
        }
        Err(e) => {
            msg!("❌ handle_callback failed: {:?}", e);
            return Err(e);
        }
    };
    
    // Get and validate program accounts based on Bonsol's account ordering
    msg!("\n🔍 Account Validation");
    
    // Bonsol prepends: requester(0), execution(1), callback_program(2), prover(3)
    // Our extra accounts start at index 4
    let requester_account = &accounts[0];
    let execution_account = &accounts[1];
    let callback_program = &accounts[2];
    let prover = &accounts[3];
    let execution_pda = &accounts[4];  // First extra account
    let hexagram_account = &accounts[5];  // Second extra account
    let system_program = &accounts[6];    // Third extra account
    
    msg!("\nAccount Index Verification:");
    msg!("Requester Account (index 0): {}", requester_account.key);
    msg!("  Is Signer: {}", requester_account.is_signer);
    msg!("  Is Writable: {}", requester_account.is_writable);
    
    msg!("Execution Account (index 1): {}", execution_account.key);
    msg!("  Is Signer: {}", execution_account.is_signer);
    msg!("  Is Writable: {}", execution_account.is_writable);
    
    msg!("Callback Program (index 2): {}", callback_program.key);
    msg!("  Is Executable: {}", callback_program.executable);
    
    msg!("Prover (index 3): {}", prover.key);
    msg!("  Is Signer: {}", prover.is_signer);
    
    msg!("Execution PDA (index 4): {}", execution_pda.key);
    msg!("  Is Signer: {}", execution_pda.is_signer);
    msg!("  Is Writable: {}", execution_pda.is_writable);
    
    msg!("Hexagram Account (index 5): {}", hexagram_account.key);
    msg!("  Is Signer: {}", hexagram_account.is_signer);
    msg!("  Is Writable: {}", hexagram_account.is_writable);
    
    msg!("System Program (index 6): {}", system_program.key);
    msg!("  Is Executable: {}", system_program.executable);
    msg!("  Address matches: {}", system_program.key == &SYSTEM_PROGRAM_ID);
    
    // Validate execution PDA
    if !execution_pda.is_signer {
        msg!("❌ Validation failed: Execution PDA must be a signer");
        msg!("Account: {}", execution_pda.key);
        msg!("Is Signer: {}", execution_pda.is_signer);
        return Err(CallbackError::InvalidSigner.into());
    }
    
    // Validate hexagram account
    if !hexagram_account.is_writable {
        msg!("❌ Validation failed: Hexagram account must be writable");
        msg!("Account: {}", hexagram_account.key);
        msg!("Is Writable: {}", hexagram_account.is_writable);
        return Err(CallbackError::InvalidInstruction.into());
    }
    
    // Verify system program
    if !system_program.executable || system_program.key != &SYSTEM_PROGRAM_ID {
        msg!("❌ Validation failed: Invalid system program");
        msg!("Expected: {}", SYSTEM_PROGRAM_ID);
        msg!("Got: {}", system_program.key);
        msg!("Is Executable: {}", system_program.executable);
        return Err(CallbackError::InvalidSystemProgram.into());
    }
    
    // Process committed outputs
    msg!("\n🔍 Processing Committed Outputs");
    let outputs = &callback_data.committed_outputs;
    msg!("Output size: {} bytes", outputs.len());
    msg!("Expected size: 86 bytes");
    
    // Validate output size
    if outputs.len() != 86 {
        msg!("❌ Invalid output size");
        msg!("Expected: 86 bytes");
        msg!("Got: {} bytes", outputs.len());
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Validate input digest
    msg!("\n🔍 Validating Input Digest");
    msg!("Expected digest length: 32 bytes");
    msg!("Actual digest length: {} bytes", callback_data.input_digest.len());
    
    if &outputs[..32] != callback_data.input_digest {
        msg!("❌ Input digest mismatch");
        msg!("Expected: {:?}", callback_data.input_digest);
        msg!("Got: {:?}", &outputs[..32]);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Validate marker
    msg!("\n🔍 Validating Marker");
    msg!("Expected marker: 0xaa");
    msg!("Got marker: 0x{:02x}", outputs[32]);
    
    if outputs[32] != 0xaa {
        msg!("❌ Invalid marker");
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Process line values
    msg!("\n🔍 Processing Line Values");
    let mut lines = [0u8; 6];
    lines.copy_from_slice(&outputs[33..39]);
    
    msg!("Line values: {:?}", lines);
    let valid_lines = lines.iter().all(|&x| (6..=9).contains(&x));
    
    if !valid_lines {
        msg!("❌ Invalid line values");
        msg!("Values must be between 6 and 9");
        msg!("Got: {:?}", lines);
        return Err(CallbackError::InvalidHexagramData.into());
    }
    
    // Process ASCII art
    let ascii_art = String::from_utf8_lossy(&outputs[39..]).to_string();
    msg!("\n🔍 ASCII Art Validation");
    msg!("ASCII art length: {} bytes", ascii_art.len());
    msg!("ASCII art content:\n{}", ascii_art);
    
    // Create hexagram account if needed
    if hexagram_account.data_is_empty() {
        msg!("\n📝 Creating New Hexagram Account");
        let rent = Rent::get()?;
        let space = 8 + 6 + 1024 + 8 + 1;
        let lamports = rent.minimum_balance(space);
        
        msg!("Account Space: {} bytes", space);
        msg!("Required Lamports: {}", lamports);
        
        msg!("Creating account...");
        let create_account_ix = system_instruction::create_account(
            execution_account.key,
            hexagram_account.key,
            lamports,
            space as u64,
            program_id,
        );
        
        solana_program::program::invoke(
            &create_account_ix,
            &[
                execution_account.clone(),
                hexagram_account.clone(),
                system_program.clone(),
            ],
        )?;
        msg!("✓ Account created successfully");
    }
    
    // Store hexagram data
    msg!("\n📝 Storing Hexagram Data");
    let hexagram = HexagramData {
        lines,
        ascii_art,
        timestamp: solana_program::clock::Clock::get()?.unix_timestamp,
        is_initialized: true,
    };
    
    msg!("Serializing data...");
    let mut data = hexagram_account.try_borrow_mut_data()?;
    hexagram.serialize(&mut *data)?;
    
    msg!("\n✅ 8BitOracle hexagram data stored successfully");
    msg!("================================================================");
    Ok(())
} 