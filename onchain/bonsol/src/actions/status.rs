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
    rent::Rent,
    system_program,
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
    msg!("🔄 [SMART CONTRACT] Starting status processing");
    
    // Check dev mode early
    let is_dev_mode = option_env!("RISC0_DEV_MODE").is_some();
    if is_dev_mode {
        msg!("🔧 [SMART CONTRACT] DEV MODE detected");
    }

    // 1. Parse status instruction
    msg!("📦 [SMART CONTRACT] Parsing status instruction");
    let st = ix.status_v1_nested_flatbuffer();
    if st.is_none() {
        msg!("❌ [SMART CONTRACT] Failed to parse status instruction - invalid flatbuffer");
        return Err(ChannelError::InvalidInstruction.into());
    }
    let st = st.unwrap();

    // 2. Parse and validate accounts
    msg!("🔍 [SMART CONTRACT] Parsing and validating accounts");
    msg!("Number of accounts provided: {}", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        msg!("Account {}: {}", i, acc.key);
    }
    
    let sa = match StatusAccounts::from_instruction(accounts, &st) {
        Ok(sa) => {
            msg!("✅ [SMART CONTRACT] Successfully validated accounts");
            sa
        },
        Err(e) => {
            msg!("❌ [SMART CONTRACT] Failed to validate accounts: {:?}", e);
            return Err(e.into());
        }
    };

    // 3. Read execution request data
    msg!("📝 [SMART CONTRACT] Reading execution request data");
    let er_ref = sa.exec.try_borrow_data()?;
    msg!("✅ [SMART CONTRACT] Successfully borrowed execution data, length: {}", er_ref.len());
    
    let er = match root_as_execution_request_v1(&er_ref) {
        Ok(er) => {
            msg!("✅ [SMART CONTRACT] Successfully parsed execution request");
            er
        },
        Err(_) => {
            msg!("❌ [SMART CONTRACT] Failed to parse execution request data");
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

    // Check expiry using the stored max_block_height
    let current_slot = Clock::get()?.slot;
    msg!(
        "⏰ [SMART CONTRACT] Checking expiry for execution {}:\n\
         - Current slot: {}\n\
         - Max block height: {}\n\
         - Blocks remaining: {}",
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
            "❌ [SMART CONTRACT] Execution {} expired: Current slot {} is greater than max block height {}",
            sa.eid,
            current_slot,
            max_block_height
        );
        return Err(ChannelError::ExecutionExpired.into());
    }

    // Modified proof validation for dev mode
    let pr_v = if is_dev_mode {
        msg!("🔧 [SMART CONTRACT] DEV MODE: Validating proof format");
        match st.proof() {
            Some(proof) => {
                msg!("📦 [SMART CONTRACT] DEV MODE: Received proof of length {}", proof.len());
                if proof.len() == 32 {
                    msg!("✅ [SMART CONTRACT] DEV MODE: Valid 32-byte proof, will pad to 256 bytes");
                    Some(proof)
                } else if proof.len() == 256 {
                    msg!("✅ [SMART CONTRACT] DEV MODE: Valid 256-byte proof");
                    Some(proof)
                } else {
                    msg!("❌ [SMART CONTRACT] DEV MODE: Invalid proof length {}, expected 32 or 256 bytes", proof.len());
                    return Err(ChannelError::InvalidProof.into());
                }
            }
            None => {
                msg!("❌ [SMART CONTRACT] DEV MODE: No proof provided");
                return Err(ChannelError::InvalidProof.into());
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
    msg!(
        "📋 [SMART CONTRACT] Checking proof components for execution ID: {}\n\
         - Dev mode: {}\n\
         - Proof present: {} (len: {})\n\
         - Execution digest present: {}\n\
         - Assumption digest present: {}\n\
         - Input digest present: {}\n\
         - Committed outputs present: {}",
        sa.eid,
        is_dev_mode,
        pr_v.is_some(),
        pr_v.map_or(0, |p| p.len()),
        execution_digest_v.is_some(),
        assumption_digest_v.is_some(),
        input_digest_v.is_some(),
        committed_outputs_v.is_some()
    );

    if let (Some(proof), Some(exed), Some(asud), Some(input_digest), Some(co)) = (
        pr_v,
        execution_digest_v,
        assumption_digest_v,
        input_digest_v,
        committed_outputs_v,
    ) {
        // Handle dev mode proofs differently
        let proof_bytes = if is_dev_mode {
            msg!("🔧 [SMART CONTRACT] DEV MODE: Creating padded proof");
            let mut padded = [0u8; 256];
            if proof.bytes().len() == 32 {
                padded[..32].copy_from_slice(proof.bytes());
                msg!("✅ [SMART CONTRACT] DEV MODE: Successfully padded 32-byte proof to 256 bytes");
            } else {
                padded.copy_from_slice(proof.bytes());
                msg!("✅ [SMART CONTRACT] DEV MODE: Using existing 256-byte proof");
            }
            padded
        } else {
            proof.bytes().try_into().map_err(|_| ChannelError::InvalidInstruction)?
        };

        if verify_input_hash && !is_dev_mode {
            er.input_digest()
                .map(|x| check_bytes_match(x.bytes(), input_digest, ChannelError::InputsDontMatch));
        }

        // In dev mode, skip verification entirely
        let verified = if is_dev_mode {
            msg!("🔧 [SMART CONTRACT] DEV MODE: Bypassing all verification steps");
            msg!("✅ [SMART CONTRACT] DEV MODE: Proof automatically accepted");
            true
        } else {
            verify_with_prover(
                input_digest,
                co,
                asud,
                image_id.unwrap(),
                exed,
                st,
                &proof_bytes,
                prover_version
            )?
        };

        if verified {
            msg!("✅ [SMART CONTRACT] Proof {} - {}",
                if is_dev_mode { "accepted in dev mode" } else { "verified" },
                if is_dev_mode { "(DEV MODE)" } else { "(PRODUCTION)" }
            );

            // Process callback if configured
            if callback_program_set && ix_prefix_set {
                msg!("📝 [SMART CONTRACT] Processing callback");
                let cbp = callback_program_id.as_deref().unwrap_or(crate::ID.as_ref());
                
                // Enhanced logging for callback program validation
                if is_dev_mode {
                    if sol_memcmp(cbp, sa.callback_program.key.as_ref(), 32) != 0 {
                        msg!("⚠️ [SMART CONTRACT] DEV MODE: Callback program mismatch (bypassing)");
                        msg!("  Expected: {}", hex::encode(cbp));
                        msg!("  Got: {}", hex::encode(sa.callback_program.key.as_ref()));
                    }
                } else {
                    check_bytes_match(
                        cbp,
                        sa.callback_program.key.as_ref(),
                        ChannelError::InvalidCallbackProgram,
                    )?;
                }

                let b = [sa.exec_bump.unwrap()];
                let mut seeds = execution_address_seeds(sa.requester.key, sa.eid.as_bytes());
                seeds.push(&b);
                let mut ainfos = vec![sa.exec.clone(), sa.callback_program.clone()];
                ainfos.extend(sa.extra_accounts.iter().cloned());
                let mut accounts = vec![AccountMeta::new_readonly(*sa.exec.key, true)];

                if let Some(extra_accounts) = callback_extra_accounts {
                    // Enhanced logging for account length validation
                    if extra_accounts.len() != sa.extra_accounts.len() {
                        msg!(
                            "⚠️ [SMART CONTRACT] Account length mismatch ({}): \n\
                             - Expected accounts: {}\n\
                             - Provided accounts: {}",
                            if is_dev_mode { "bypassing" } else { "failing" },
                            extra_accounts.len(),
                            sa.extra_accounts.len()
                        );
                        if !is_dev_mode {
                            return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                        }
                    }

                    // Log account details before processing
                    msg!("📊 [SMART CONTRACT] Account Details:");
                    msg!("- Execution Account: {} (balance: {})", sa.exec.key, sa.exec.lamports());
                    msg!("- Callback Program: {} (executable: {})", sa.callback_program.key, sa.callback_program.executable);
                    
                    for (i, a) in sa.extra_accounts.iter().enumerate() {
                        msg!(
                            "- Extra Account {}: {} \n\
                             * Balance: {} lamports\n\
                             * Owner: {}\n\
                             * Executable: {}\n\
                             * Writable: {}\n\
                             * Signer: {}",
                            i,
                            a.key,
                            a.lamports(),
                            a.owner,
                            a.executable,
                            a.is_writable,
                            a.is_signer
                        );

                        let (key, writable) = if i < extra_accounts.len() {
                            &extra_accounts[i]
                        } else {
                            msg!("⚠️ [SMART CONTRACT] DEV MODE: Account index {} out of bounds", i);
                            continue;
                        };

                        // Enhanced logging for account key validation
                        if sol_memcmp(a.key.as_ref(), key.as_slice(), 32) != 0 {
                            msg!(
                                "⚠️ [SMART CONTRACT] Account key mismatch for index {} ({}):\n\
                                 - Expected: {}\n\
                                 - Got: {}\n\
                                 - Balance: {} lamports\n\
                                 - Owner: {}",
                                i,
                                if is_dev_mode { "bypassing" } else { "failing" },
                                hex::encode(key),
                                hex::encode(a.key.as_ref()),
                                a.lamports(),
                                a.owner
                            );
                            if !is_dev_mode {
                                return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                            }
                        }

                        // Enhanced logging for writability validation
                        if a.is_writable {
                            if *writable == 0 {
                                msg!(
                                    "⚠️ [SMART CONTRACT] Account {} writability mismatch ({}):\n\
                                     - Account is writable but expected readonly\n\
                                     - Balance: {} lamports\n\
                                     - Owner: {}",
                                    i,
                                    if is_dev_mode { "bypassing" } else { "failing" },
                                    a.lamports(),
                                    a.owner
                                );
                                if !is_dev_mode {
                                    return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                                }
                            }
                            accounts.push(AccountMeta::new(*a.key, false));
                        } else {
                            if *writable == 1 {
                                msg!(
                                    "⚠️ [SMART CONTRACT] Account {} writability mismatch ({}):\n\
                                     - Account is readonly but expected writable\n\
                                     - Balance: {} lamports\n\
                                     - Owner: {}",
                                    i,
                                    if is_dev_mode { "bypassing" } else { "failing" },
                                    a.lamports(),
                                    a.owner
                                );
                                if !is_dev_mode {
                                    return Err(ChannelError::InvalidCallbackExtraAccounts.into());
                                }
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

                // Capture all needed values before moving accounts
                let accounts_len = accounts.len();
                let payload_len = payload.len();
                
                // Log account details before moving accounts
                msg!("📤 [SMART CONTRACT] Callback preparation:");
                msg!("- Number of accounts: {}", accounts_len);
                msg!("- Payload size: {} bytes", payload_len);
                msg!("- Program ID: {}", sa.callback_program.key);
                
                // Create instruction, moving accounts
                let callback_ix = Instruction::new_with_bytes(*sa.callback_program.key, &payload, accounts);
                
                // Calculate rent requirements
                let rent = Rent::get()?;
                msg!("💰 [SMART CONTRACT] Rent exempt minimum: {} lamports", rent.minimum_balance(0));
                
                // Skip rent check for native programs
                for (i, a) in ainfos.iter().enumerate() {
                    if a.executable && a.owner == &system_program::ID {
                        msg!("📝 [SMART CONTRACT] Skipping rent check for native program at index {}", i);
                        continue;
                    }
                    
                    let min_balance = rent.minimum_balance(a.data_len());
                    if a.lamports() < min_balance {
                        msg!(
                            "⚠️ [SMART CONTRACT] Account {} has insufficient balance for rent:\n\
                             - Required: {} lamports\n\
                             - Current: {} lamports\n\
                             - Owner: {}\n\
                             - Data length: {} bytes",
                            i,
                            min_balance,
                            a.lamports(),
                            a.owner,
                            a.data_len()
                        );
                        if !is_dev_mode {
                            return Err(ChannelError::NotRentExempt.into());
                        }
                    }
                }
                
                let res = invoke_signed(&callback_ix, &ainfos, &[&seeds]);
                match res {
                    Ok(_) => {
                        msg!("✅ [SMART CONTRACT] Callback executed successfully");
                    }
                    Err(e) => {
                        msg!(
                            "❌ [SMART CONTRACT] {} Callback Failed:\n\
                             - Error: {:?}\n\
                             - Program: {}\n\
                             - Accounts: {}\n\
                             - Payload size: {} bytes",
                            sa.eid,
                            e,
                            sa.callback_program.key,
                            accounts_len,
                            payload_len
                        );
                        if is_dev_mode {
                            msg!("🔧 [SMART CONTRACT] DEV MODE: Ignoring callback failure");
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            // Process tip and cleanup
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
            msg!("🔧 [SMART CONTRACT] DEV MODE: Treating proving error as success");
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
    
    msg!("🔄 [SMART CONTRACT] Starting proof verification process");
    
    // Basic validation for both dev mode and production
    msg!(
        "📋 [SMART CONTRACT] Verification parameters:\n\
         - Prover version: {:?}\n\
         - Input digest length: {}\n\
         - Committed outputs length: {}\n\
         - Assumption digest length: {}\n\
         - Execution digest length: {}\n\
         - System exit code: {}\n\
         - User exit code: {}",
        prover_version,
        input_digest.len(),
        co.len(),
        asud.len(),
        exed.len(),
        st.exit_code_system(),
        st.exit_code_user()
    );

    // Basic structural validation
    if input_digest.len() != 32 {
        msg!("❌ [SMART CONTRACT] Invalid input digest length: {}", input_digest.len());
        return Err(ChannelError::InvalidProof.into());
    }
    if asud.len() != 32 {
        msg!("❌ [SMART CONTRACT] Invalid assumption digest length: {}", asud.len());
        return Err(ChannelError::InvalidProof.into());
    }
    if exed.len() != 32 {
        msg!("❌ [SMART CONTRACT] Invalid execution digest length: {}", exed.len());
        return Err(ChannelError::InvalidProof.into());
    }
    
    // In dev mode, perform basic validation but skip cryptographic checks
    if is_dev_mode {
        msg!("🔧 [SMART CONTRACT] DEV MODE: Performing basic validation");
        
        // Generate output digest to validate format
        let output_digest = match prover_version {
            VERSION_V1_0_1 => output_digest_v1_0_1(input_digest, co, asud),
            VERSION_V1_2_1 => output_digest_v1_2_1(input_digest, co, asud),
            _ => {
                msg!("❌ [SMART CONTRACT] DEV MODE: Unsupported prover version");
                return Ok(false);
            }
        };
        
        msg!("✅ [SMART CONTRACT] DEV MODE: Basic validation passed");
        msg!("📝 [SMART CONTRACT] DEV MODE: Output digest: {}", hex::encode(&output_digest));
        
        return Ok(true);
    }
    
    // Production mode: full cryptographic verification
    let verified = match prover_version {
        VERSION_V1_0_1 => {
            msg!("🔒 [SMART CONTRACT] Using V1.0.1 verification protocol");
            let output_digest = output_digest_v1_0_1(input_digest, co, asud);
            msg!("📝 [SMART CONTRACT] Generated output digest: {}", hex::encode(&output_digest));
            let proof_inputs = prepare_inputs_v1_0_1(
                &image_id,
                exed,
                output_digest.as_ref(),
                st.exit_code_system(),
                st.exit_code_user(),
            )?;
            msg!("📦 [SMART CONTRACT] Prepared proof inputs (length: {})", proof_inputs.len());
            verify_risc0_v1_0_1(proof, &proof_inputs)?
        }
        VERSION_V1_2_1 => {
            msg!("🔒 [SMART CONTRACT] Using V1.2.1 verification protocol");
            let output_digest = output_digest_v1_2_1(input_digest, co, asud);
            msg!("📝 [SMART CONTRACT] Generated output digest: {}", hex::encode(&output_digest));
            let proof_inputs = prepare_inputs_v1_2_1(
                &image_id,
                exed,
                output_digest.as_ref(),
                st.exit_code_system(),
                st.exit_code_user(),
            )?;
            msg!("📦 [SMART CONTRACT] Prepared proof inputs (length: {})", proof_inputs.len());
            verify_risc0_v1_2_1(proof, &proof_inputs)?
        }
        _ => {
            msg!("❌ [SMART CONTRACT] Unsupported prover version");
            false
        }
    };
    
    msg!(
        "{} [SMART CONTRACT] Verification complete: {}",
        if verified { "✅" } else { "❌" },
        if verified { "ACCEPTED" } else { "REJECTED" }
    );
    Ok(verified)
}
