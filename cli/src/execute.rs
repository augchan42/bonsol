use crate::common::*;
use anyhow::Result;
use bonsol_prover::input_resolver::{DefaultInputResolver, InputResolver, ProgramInput};
use bonsol_sdk::instructions::{ExecutionConfig, InputRef};
use bonsol_sdk::{BonsolClient, ExecutionAccountStatus, InputType};
use bonsol_interface::bonsol_schema::ExitCode;
use indicatif::ProgressBar;
use log::{debug, error, info, warn};
use sha2::{Digest, Sha256};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::fs::File;
use std::sync::Arc;
use tokio::time::Instant;
use std::env;

pub async fn execution_waiter(
    sdk: &BonsolClient,
    requester: Pubkey,
    execution_id: String,
    expiry: u64,
    timeout: Option<u64>,
) -> Result<()> {
    let is_dev_mode = env::var("RISC0_DEV_MODE").is_ok();
    let indicator = ProgressBar::new_spinner();

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    let now = Instant::now();
    
    info!("Starting execution waiter for ID: {}", execution_id);
    debug!("Parameters: requester={}, expiry={}, timeout={:?}", requester, expiry, timeout);
    
    loop {
        if let Some(timeout) = timeout {
            if now.elapsed().as_secs() > timeout {
                error!("Execution timed out after {} seconds", timeout);
                return Err(anyhow::anyhow!("Timeout"));
            }
        }
        interval.tick().await;

        let current_block = sdk.get_current_slot().await?;
        indicator.set_message(format!(
            "Waiting for execution to be claimed, current block {} expiry {}",
            current_block, expiry
        ));
        if current_block > expiry {
            error!("Execution expired at block {}, current block {}", expiry, current_block);
            indicator.finish_with_message("Execution expired");
            return Err(anyhow::anyhow!("Execution expired"));
        }

        let claim_state = sdk.get_claim_state_v1(&requester, &execution_id).await;
        if let Ok(claim_state) = claim_state {
            let claim = claim_state.claim()?;
            info!(
                "Execution claimed by {} at slot {}, commitment {}",
                bs58::encode(claim.claimer).into_string(),
                claim.claimed_at,
                claim.block_commitment
            );
            indicator.finish_with_message(format!(
                "Claimed by {} at slot {}, committed {}",
                bs58::encode(claim.claimer).into_string(),
                claim.claimed_at,
                claim.block_commitment
            ));
            break;
        } else if let Err(e) = claim_state {
            debug!("Waiting for claim: {}", e);
        }
    }
    
    info!("Claim found, waiting for execution completion");
    //now we are looking for execution request finished
    loop {
        if let Some(timeout) = timeout {
            if now.elapsed().as_secs() > timeout {
                error!("Execution completion timed out");
                indicator.finish_with_message("Execution timed out");
                return Err(anyhow::anyhow!("Timeout"));
            }
        }
        interval.tick().await;
        let exec_status = sdk
            .get_execution_request_v1(&requester, &execution_id)
            .await?;
        match exec_status {
            ExecutionAccountStatus::Completed(mut ec) => {
                if is_dev_mode && ec == ExitCode::ProvingError {
                    info!("Dev mode: treating ProvingError as success");
                    ec = ExitCode::Success;
                }
                info!("Execution completed with exit code {}", ec);
                
                // Get the raw account data using our new method
                if let Some(account_data) = sdk.get_execution_account_data(&requester, &execution_id).await? {
                    debug!("Raw account data inspection:");
                    debug!("  - Total size: {} bytes", account_data.len());
                    debug!("  - First byte (exit code): {:#04x}", account_data[0]);
                    
                    // For completed executions, data after the first byte is our journal
                    if account_data.len() > 33 { // 1 byte exit code + 32 bytes input digest
                        let journal_data = &account_data[1..];
                        let (input_digest, committed_outputs) = journal_data.split_at(32);
                        
                        debug!("Journal data breakdown:");
                        debug!("  - Input digest: {} bytes", input_digest.len());
                        debug!("  - Input digest (hex): {:02x?}", input_digest);
                        debug!("  - Committed outputs: {} bytes", committed_outputs.len());
                        debug!("  - First 32 bytes of outputs: {:02x?}", &committed_outputs[..32.min(committed_outputs.len())]);
                        
                        // Try to find ASCII art (it should be after the structured output)
                        if let Some(marker_pos) = committed_outputs.iter().position(|&x| x == 0xAA) {
                            debug!("Found success marker 0xAA at position {}", marker_pos);
                            debug!("Data after marker: {} bytes", committed_outputs.len() - marker_pos);
                            
                            // Skip marker byte and 6 line values
                            if committed_outputs.len() > marker_pos + 7 {
                                let line_values = &committed_outputs[marker_pos + 1..marker_pos + 7];
                                let ascii_art_bytes = &committed_outputs[marker_pos + 7..];
                                
                                debug!("Line values: {:02x?}", line_values);
                                debug!("ASCII art section: {} bytes", ascii_art_bytes.len());
                                
                                let ascii_art = String::from_utf8_lossy(ascii_art_bytes);
                                info!("Found ASCII art output:\n{}", ascii_art);
                                indicator.finish_with_message(format!("Execution completed with exit code {}\n\nHexagram:\n{}", ec, ascii_art));
                                return Ok(());
                            } else {
                                warn!("Insufficient data after marker: expected at least 7 bytes for line values and ASCII art");
                                warn!("  - Bytes available after marker: {} bytes", committed_outputs.len() - marker_pos);
                                warn!("  - Expected structure:");
                                warn!("    • Marker (0xAA): 1 byte");
                                warn!("    • Line values: 6 bytes");
                                warn!("    • ASCII art: remaining bytes");
                            }
                        } else {
                            warn!("No success marker (0xAA) found in output");
                            warn!("First 16 bytes of committed outputs: {:02x?}", &committed_outputs[..16.min(committed_outputs.len())]);
                        }
                        
                        // If we found no ASCII art, print the raw bytes for debugging
                        debug!("Raw committed outputs: {:02x?}", committed_outputs);
                        indicator.finish_with_message(format!(
                            "Execution completed with exit code {}.\nRaw committed outputs: {:02x?}", 
                            ec,
                            committed_outputs
                        ));
                        return Ok(());
                    } else {
                        warn!("Account data too small: {} bytes", account_data.len());
                        warn!("Expected minimum size: 33 bytes (1 byte exit code + 32 bytes input digest)");
                        if account_data.len() > 0 {
                            warn!("Available data: {:02x?}", account_data);
                        }
                    }
                } else {
                    error!("Failed to get execution account data");
                    error!("This could indicate the account was not created or was closed");
                }
                
                indicator.finish_with_message(format!("Execution completed with exit code {}", ec));
                return Ok(());
            }
            ExecutionAccountStatus::Pending(req) => {
                debug!(
                    "Execution still pending: image_id={}, tip={}, expiry={}",
                    req.image_id.unwrap_or_else(|| "unknown".to_string()),
                    req.tip,
                    req.max_block_height
                );
                indicator.tick();
                continue;
            }
        }
    }
}

pub async fn execute(
    sdk: &BonsolClient,
    rpc_url: String,
    keypair: impl Signer,
    execution_request_file: Option<String>,
    image_id: Option<String>,
    execution_id: Option<String>,
    timeout: Option<u64>,
    inputs_file: Option<String>,
    tip: Option<u64>,
    expiry: Option<u64>,
    stdin: Option<String>,
    wait: bool,
) -> Result<()> {
    let indicator = ProgressBar::new_spinner();
    
    info!("Starting execution process");
    debug!(
        "Parameters: rpc_url={}, image_id={:?}, execution_id={:?}, timeout={:?}, tip={:?}, expiry={:?}, wait={}",
        rpc_url, image_id, execution_id, timeout, tip, expiry, wait
    );
    
    let erstr =
        execution_request_file.ok_or(anyhow::anyhow!("Execution request file not provided"))?;
    let erfile = File::open(erstr)?;
    let execution_request_file: ExecutionRequestFile = serde_json::from_reader(erfile)?;
    
    debug!("Loaded execution request file");
    
    let inputs = if let Some(inputs) = execution_request_file.inputs {
        inputs
    } else {
        debug!("Getting inputs from file or stdin");
        execute_get_inputs(inputs_file, stdin)?
    };
    
    let execution_id = execution_id
        .or(execution_request_file.execution_id)
        .or(Some(rand_id(8)))
        .ok_or(anyhow::anyhow!("Execution id not provided"))?;
        
    info!("Using execution ID: {}", execution_id);
    
    let image_id = image_id
        .or(execution_request_file.image_id)
        .ok_or(anyhow::anyhow!("Image id not provided"))?;
        
    info!("Using image ID: {}", image_id);
    
    let tip = tip
        .or(execution_request_file.tip)
        .ok_or(anyhow::anyhow!("Tip not provided"))?;
    let expiry = expiry
        .or(execution_request_file.expiry)
        .ok_or(anyhow::anyhow!("Expiry not provided"))?;
    let callback_config = execution_request_file.callback_config;
    
    if let Some(ref cb) = callback_config {
        info!(
            "Using callback configuration: program_id={:?}, instruction_prefix={:?}, extra_accounts={}",
            cb.program_id,
            cb.instruction_prefix,
            cb.extra_accounts.as_ref().map(|a| a.len()).unwrap_or(0)
        );
    }
    
    let mut input_hash =
        if let Some(input_hash) = execution_request_file.execution_config.input_hash {
            debug!("Using provided input hash: {}", input_hash);
            hex::decode(&input_hash)
                .map_err(|_| anyhow::anyhow!("Invalid input hash, must be hex encoded"))?
        } else {
            vec![]
        };

    let signer = keypair.pubkey();
    info!("Using signer: {}", signer);
    
    let transformed_inputs = execute_transform_cli_inputs(inputs)?;
    debug!("Transformed {} inputs", transformed_inputs.len());
    
    let verify_input_hash = execution_request_file
        .execution_config
        .verify_input_hash
        .unwrap_or(false);
    let hash_inputs = verify_input_hash
        && transformed_inputs.iter().all(|i| i.input_type != InputType::Private);
        
    debug!("verify_input_hash={}, hash_inputs={}", verify_input_hash, hash_inputs);
    
    if hash_inputs {
        indicator.set_message("Getting/Hashing inputs");
        info!("Calculating input hash");
        
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));
        let input_resolver =
            DefaultInputResolver::new(Arc::new(reqwest::Client::new()), rpc_client);
        let hashing_inputs = input_resolver
            .resolve_public_inputs(transformed_inputs.clone())
            .await?;
            
        debug!("Resolved {} inputs for hashing", hashing_inputs.len());
        
        let mut hash = Sha256::new();
        for input in hashing_inputs {
            if let ProgramInput::Resolved(ri) = input {
                hash.update(&ri.data);
            } else {
                error!("Found unresolved input during hashing");
                return Err(anyhow::anyhow!("Unresolved input"));
            }
        }
        input_hash = hash.finalize().to_vec();
        debug!("Calculated input hash: {:?}", input_hash);
    }
    
    let execution_config = ExecutionConfig {
        verify_input_hash,
        input_hash: Some(&input_hash),
        forward_output: execution_request_file
            .execution_config
            .forward_output
            .unwrap_or(false),
    };
    
    let current_block = sdk.get_current_slot().await?;
    let expiry = expiry + current_block;
    info!("Current block: {}, execution expiry: {}", current_block, expiry);
    
    println!("Execution expiry {}", expiry);
    println!("current block {}", current_block);
    indicator.set_message("Building transaction");
    
    info!("Building execution transaction");
    let ixs = sdk
        .execute_v1(
            &signer,
            &image_id,
            &execution_id,
            transformed_inputs
                .iter()
                .map(|i| InputRef::new(i.input_type, i.data.as_deref().unwrap_or_default()))
                .collect(),
            tip,
            expiry,
            execution_config,
            callback_config.map(|c| c.into()),
            None, // A future cli change can implement prover version selection
        )
        .await?;
        
    debug!("Built {} instructions", ixs.len());
    indicator.finish_with_message("Sending transaction");
    
    info!("Sending transaction");
    sdk.send_txn_standard(&keypair, ixs).await?;
    info!("Transaction sent successfully");
    
    indicator.finish_with_message("Waiting for execution");
    if wait {
        info!("Waiting for execution completion");
        execution_waiter(sdk, keypair.pubkey(), execution_id, expiry, timeout).await
    } else {
        info!("Not waiting for execution completion");
        Ok(())
    }
}
