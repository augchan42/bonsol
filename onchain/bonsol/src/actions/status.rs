use crate::{
    assertions::*,
    error::ChannelError,
    proof_handling::{
        output_digest_v1_0_1, output_digest_v1_2_1, prepare_inputs_v1_0_1, prepare_inputs_v1_2_1,
        verify_risc0_v1_0_1, verify_risc0_v1_2_1,
    },
    utilities::*,
};

use bonsol_interface::{
    bonsol_schema::{
        root_as_execution_request_v1, ChannelInstruction, ExitCode, StatusV1,
    },
    prover_version::{ProverVersion, VERSION_V1_0_1, VERSION_V1_2_1},
    util::execution_address_seeds,
};

use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_memory::sol_memcmp,
    sysvar::Sysvar,
};

use hex;

struct StatusAccounts<'a, 'b> {
    pub requester: &'a AccountInfo<'a>,
    pub exec: &'a AccountInfo<'a>,
    pub prover: &'a AccountInfo<'a>,
    pub callback_program: &'a AccountInfo<'a>,
    pub extra_accounts: &'a [AccountInfo<'a>],
    pub exec_bump: Option<u8>,
    pub eid: &'b str,
}

impl<'a, 'b> StatusAccounts<'a, 'b> {
    fn from_instruction(
        accounts: &'a [AccountInfo<'a>],
        data: &'b StatusV1<'b>,
    ) -> Result<Self, ChannelError> {
        msg!("Starting account validation");
        
        if accounts.len() < 4 {
            msg!("Not enough accounts provided. Expected at least 4, got {}", accounts.len());
            return Err(ChannelError::InvalidExecutionAccount);
        }
        
        let ea = &accounts[1];
        let prover = &accounts[3];
        let callback_program = &accounts[2];
        
        msg!("Extracting execution ID");
        let eid = match data.execution_id() {
            Some(id) => {
                msg!("Found execution ID: {}", id);
                id
            },
            None => {
                msg!("Missing execution ID in status data");
                return Err(ChannelError::InvalidExecutionAccount);
            }
        };
        
        msg!("Checking PDA derivation");
        msg!("Requester key: {}", accounts[0].key);
        msg!("Execution account key: {}", ea.key);
        
        let bmp = Some(check_pda(
            &execution_address_seeds(accounts[0].key, eid.as_bytes()),
            ea.key,
            ChannelError::InvalidExecutionAccount,
        )?);
        msg!("PDA check passed with bump: {:?}", bmp);
        
        let stat = StatusAccounts {
            requester: &accounts[0],
            exec: &accounts[1],
            callback_program,
            prover,
            extra_accounts: &accounts[4..],
            exec_bump: bmp,
            eid,
        };
        msg!("Successfully created StatusAccounts");
        Ok(stat)
    }
}

pub fn process_status_v1<'a>(
    accounts: &'a [AccountInfo<'a>],
    ix: ChannelInstruction,
) -> Result<(), ProgramError> {
    msg!("Starting status processing");
    
    // 1. Parse status instruction
    msg!("Parsing status instruction");
    let st = ix.status_v1_nested_flatbuffer();
    if st.is_none() {
        msg!("Failed to parse status instruction - invalid flatbuffer");
        return Err(ChannelError::InvalidInstruction.into());
    }
    let st = st.unwrap();
    msg!("Successfully parsed status instruction");

    // 2. Parse and validate accounts
    msg!("Parsing and validating accounts");
    msg!("Number of accounts provided: {}", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        msg!("Account {}: {}", i, acc.key);
    }
    
    let sa = match StatusAccounts::from_instruction(accounts, &st) {
        Ok(sa) => {
            msg!("Successfully validated accounts");
            sa
        },
        Err(e) => {
            msg!("Failed to validate accounts: {:?}", e);
            return Err(e.into());
        }
    };

    // 3. Read execution request data
    msg!("Reading execution request data");
    let er_ref = sa.exec.try_borrow_data()?;
    msg!("Successfully borrowed execution data, length: {}", er_ref.len());
    
    let er = match root_as_execution_request_v1(&er_ref) {
        Ok(er) => {
            msg!("Successfully parsed execution request");
            er
        },
        Err(_) => {
            msg!("Failed to parse execution request data");
            return Err(ChannelError::InvalidExecutionAccount.into());
        }
    };

    // Extract all needed values from er before dropping er_ref
    let callback_program_set = sol_memcmp(sa.callback_program.key.as_ref(), crate::ID.as_ref(), 32) != 0;
    let ix_prefix_set = er.callback_instruction_prefix().is_some();
    let tip = er.tip();
    let forward_output = er.forward_output();
    let callback_program_id = er.callback_program_id().map(|b| b.bytes().to_vec());
    let callback_instruction_prefix = er.callback_instruction_prefix().map(|p| p.bytes().to_vec());
    let max_block_height = er.max_block_height();
    let verify_input_hash = er.verify_input_hash();
    let prover_version = ProverVersion::try_from(er.prover_version()).unwrap_or(ProverVersion::default());
    let image_id = er.image_id().map(|s| s.to_string());
    let callback_extra_accounts = if let Some(accounts) = er.callback_extra_accounts() {
        let mut acc_vec = Vec::with_capacity(accounts.len());
        for i in 0..accounts.len() {
            let acc = accounts.get(i);
            let pubkey = acc.pubkey();
            acc_vec.push((pubkey.into_iter().collect::<Vec<u8>>(), acc.writable()));
        }
        Some(acc_vec)
    } else {
        None
    };

    // Now we can safely drop er_ref as we have all the data we need
    // drop(er_ref);

    let is_dev_mode = option_env!("RISC0_DEV_MODE").is_some();
    
    // Add detailed logging about the proof
    if let Some(proof) = st.proof() {
        msg!(
            "Received proof with length: {}. Required length: {}. Execution ID: {}",
            proof.len(),
            if is_dev_mode { "32 or 256" } else { "256" },
            sa.eid
        );
    } else {
        msg!(
            "No proof provided in status update for execution ID: {}",
            sa.eid
        );
    }

    // Check expiry using the stored max_block_height
    let current_slot = Clock::get()?.slot;
    msg!(
        "Checking expiry for execution {}: Current slot: {}, Max block height: {}, Blocks remaining: {}",
        sa.eid,
        current_slot,
        max_block_height,
        if max_block_height > current_slot {
            max_block_height - current_slot
        } else {
            0
        }
    );

    if current_slot > max_block_height {
        msg!(
            "Execution {} expired: Current slot {} is greater than max block height {}",
            sa.eid,
            current_slot,
            max_block_height
        );
        return Err(ChannelError::ExecutionExpired.into());
    }

    // Modified proof validation for dev mode
    let pr_v = if is_dev_mode {
        // In dev mode, accept any proof or skip validation
        match st.proof() {
            Some(proof) if proof.len() == 32 || proof.len() == 256 => {
                msg!("Dev mode: Using provided proof of length {}", proof.len());
                Some(proof)
            }
            Some(proof) => {
                msg!("Dev mode: Invalid proof length {}, skipping validation", proof.len());
                Some(proof) // In dev mode, accept any proof length
            }
            None => {
                msg!("Dev mode: No proof provided, skipping validation");
                st.proof() // Pass through None in dev mode
            }
        }
    } else {
        // Production mode: strict 256-byte proof requirement
        st.proof().filter(|x| x.len() == 256)
    };

    let execution_digest_v = st.execution_digest().map(|x| x.bytes());
    let input_digest_v = st.input_digest().map(|x| x.bytes());
    let assumption_digest_v = st.assumption_digest().map(|x| x.bytes());
    let committed_outputs_v = st.committed_outputs().map(|x| x.bytes());

    // Add detailed diagnostic logging
    msg!("Checking proof components for execution ID: {}", sa.eid);
    msg!("  - Dev mode: {}", is_dev_mode);
    msg!("  - Proof present: {} (len: {})", pr_v.is_some(), pr_v.map_or(0, |p| p.len()));
    msg!("  - Execution digest present: {}", execution_digest_v.is_some());
    msg!("  - Assumption digest present: {}", assumption_digest_v.is_some());
    msg!("  - Input digest present: {}", input_digest_v.is_some());
    msg!("  - Committed outputs present: {}", committed_outputs_v.is_some());

    if let (Some(proof), Some(exed), Some(asud), Some(input_digest), Some(co)) = (
        pr_v,
        execution_digest_v,
        assumption_digest_v,
        input_digest_v,
        committed_outputs_v,
    ) {
        // Handle dev mode proofs differently
        let proof_bytes = if is_dev_mode {
            if proof.len() == 32 {
                // In dev mode with 32-byte proof, pad with zeros
                let mut padded = [0u8; 256];
                padded[..32].copy_from_slice(proof.bytes());
                msg!("Dev mode: Padded 32-byte proof to 256 bytes");
                padded
            } else {
                // Use the proof as is if it's already 256 bytes
                proof.bytes().try_into().map_err(|_| ChannelError::InvalidInstruction)?
            }
        } else {
            // Normal 256-byte proof requirement
            proof.bytes().try_into().map_err(|_| ChannelError::InvalidInstruction)?
        };

        if verify_input_hash {
            er.input_digest()
                .map(|x| check_bytes_match(x.bytes(), input_digest, ChannelError::InputsDontMatch));
        }
        let verified = verify_with_prover(input_digest, co, asud, image_id.unwrap(), exed, st, &proof_bytes, prover_version)?;

        if verified {
            if callback_program_set && ix_prefix_set {
                let cbp = callback_program_id.as_deref().unwrap_or(crate::ID.as_ref());
                check_bytes_match(
                    cbp,
                    sa.callback_program.key.as_ref(),
                    ChannelError::InvalidCallbackProgram,
                )?;

                let b = [sa.exec_bump.unwrap()];
                let mut seeds = execution_address_seeds(sa.requester.key, sa.eid.as_bytes());
                seeds.push(&b);
                let mut ainfos = vec![sa.exec.clone(), sa.callback_program.clone()];
                ainfos.extend(sa.extra_accounts.iter().cloned());
                let mut accounts = vec![AccountMeta::new_readonly(*sa.exec.key, true)];

                if let Some(extra_accounts) = callback_extra_accounts {
                    if extra_accounts.len() != sa.extra_accounts.len() {
                        return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                    }
                    for (i, a) in sa.extra_accounts.iter().enumerate() {
                        let (key, writable) = &extra_accounts[i];
                        if sol_memcmp(a.key.as_ref(), key.as_slice(), 32) != 0 {
                            return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                        }
                        if a.is_writable {
                            if *writable == 0 {
                                return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                            }
                            accounts.push(AccountMeta::new(*a.key, false));
                        } else {
                            if *writable == 1 {
                                return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                            }
                            accounts.push(AccountMeta::new_readonly(*a.key, false));
                        }
                    }
                }

                let payload = if forward_output && st.committed_outputs().is_some() {
                    [
                        callback_instruction_prefix.unwrap(),
                        input_digest_v.unwrap().to_vec(),
                        st.committed_outputs().unwrap().bytes().to_vec(),
                    ]
                    .concat()
                } else {
                    callback_instruction_prefix.unwrap()
                };

                let callback_ix = Instruction::new_with_bytes(*sa.callback_program.key, &payload, accounts);
                let res = invoke_signed(&callback_ix, &ainfos, &[&seeds]);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        msg!("{} Callback Failed: {:?}", sa.eid, e);
                    }
                }
            }
            payout_tip(sa.exec, sa.prover, tip)?;
            drop(er_ref);
            cleanup_execution_account(sa.exec, sa.requester, ExitCode::Success as u8, input_digest_v)?;
        } else {
            msg!("{} Verifying Failed Cleaning up", sa.eid);
            drop(er_ref);
            cleanup_execution_account(sa.exec, sa.requester, ExitCode::VerifyError as u8, input_digest_v)?;
        }
    } else {
        msg!("{} Proving Failed Cleaning up", sa.eid);
        
        // In dev mode, treat proving error as success
        if is_dev_mode {
            msg!("Dev mode: Treating proving error as success");
            if callback_program_set && ix_prefix_set {
                let cbp = callback_program_id.as_deref().unwrap_or(crate::ID.as_ref());
                check_bytes_match(
                    cbp,
                    sa.callback_program.key.as_ref(),
                    ChannelError::InvalidCallbackProgram,
                )?;

                let b = [sa.exec_bump.unwrap()];
                let mut seeds = execution_address_seeds(sa.requester.key, sa.eid.as_bytes());
                seeds.push(&b);
                let mut ainfos = vec![sa.exec.clone(), sa.callback_program.clone()];
                ainfos.extend(sa.extra_accounts.iter().cloned());
                let mut accounts = vec![AccountMeta::new_readonly(*sa.exec.key, true)];
                
                if let Some(extra_accounts) = callback_extra_accounts {
                    if extra_accounts.len() != sa.extra_accounts.len() {
                        return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                    }
                    for (i, a) in sa.extra_accounts.iter().enumerate() {
                        let (key, writable) = &extra_accounts[i];
                        if sol_memcmp(a.key.as_ref(), key.as_slice(), 32) != 0 {
                            return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                        }
                        if a.is_writable {
                            if *writable == 0 {
                                return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                            }
                            accounts.push(AccountMeta::new(*a.key, false));
                        } else {
                            if *writable == 1 {
                                return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                            }
                            accounts.push(AccountMeta::new_readonly(*a.key, false));
                        }
                    }
                }

                let payload = if forward_output && st.committed_outputs().is_some() {
                    [
                        callback_instruction_prefix.unwrap(),
                        input_digest_v.unwrap().to_vec(),
                        st.committed_outputs().unwrap().bytes().to_vec(),
                    ]
                    .concat()
                } else {
                    callback_instruction_prefix.unwrap()
                };
                
                let callback_ix =
                    Instruction::new_with_bytes(*sa.callback_program.key, &payload, accounts);
                let res = invoke_signed(&callback_ix, &ainfos, &[&seeds]);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        msg!("{} Callback Failed: {:?}", sa.eid, e);
                    }
                }
            }
            
            drop(er_ref);
            cleanup_execution_account(sa.exec, sa.requester, ExitCode::Success as u8, input_digest_v)?;
        } else {
            drop(er_ref);
            cleanup_execution_account(sa.exec, sa.requester, ExitCode::ProvingError as u8, input_digest_v)?;
        }
    }
    Ok(())
}

fn verify_with_prover(
    input_digest: &[u8],
    co: &[u8],
    asud: &[u8],
    image_id: String,
    exed: &[u8],
    st: StatusV1,
    proof: &[u8; 256],
    prover_version: ProverVersion,
) -> Result<bool, ProgramError> {
    let is_dev_mode = option_env!("RISC0_DEV_MODE").is_some();
    
    // In dev mode, skip actual verification
    if is_dev_mode {
        msg!("Dev mode: Skipping proof verification");
        return Ok(true);
    }
    
    msg!("Verifying proof with prover version: {:?}", prover_version);
    msg!("Input digest length: {}", input_digest.len());
    msg!("Committed outputs length: {}", co.len());
    msg!("Assumption digest length: {}", asud.len());
    msg!("Execution digest length: {}", exed.len());
    msg!("System exit code: {}", st.exit_code_system());
    msg!("User exit code: {}", st.exit_code_user());
    
    let verified = match prover_version {
        VERSION_V1_0_1 => {
            msg!("Using V1.0.1 verification");
            let output_digest = output_digest_v1_0_1(input_digest, co, asud);
            msg!("Generated output digest: {}", hex::encode(&output_digest));
            let proof_inputs = prepare_inputs_v1_0_1(
                &image_id,
                exed,
                output_digest.as_ref(),
                st.exit_code_system(),
                st.exit_code_user(),
            )?;
            msg!("Prepared proof inputs length: {}", proof_inputs.len());
            verify_risc0_v1_0_1(proof, &proof_inputs)?
        }
        VERSION_V1_2_1 => {
            msg!("Using V1.2.1 verification");
            let output_digest = output_digest_v1_2_1(input_digest, co, asud);
            msg!("Generated output digest: {}", hex::encode(&output_digest));
            let proof_inputs = prepare_inputs_v1_2_1(
                &image_id,
                exed,
                output_digest.as_ref(),
                st.exit_code_system(),
                st.exit_code_user(),
            )?;
            msg!("Prepared proof inputs length: {}", proof_inputs.len());
            verify_risc0_v1_2_1(proof, &proof_inputs)?
        }
        _ => {
            msg!("Unsupported prover version");
            false
        }
    };
    
    msg!("Proof verification result: {}", verified);
    Ok(verified)
}
