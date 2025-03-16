use std::sync::Arc;

use {
    async_trait::async_trait,
    bonsol_interface::{
        bonsol_schema::{
            ChannelInstruction, ChannelInstructionArgs, ChannelInstructionIxType, ClaimV1,
            ClaimV1Args, StatusTypes, StatusV1, StatusV1Args,
        },
        util::{deployment_address, execution_address, execution_claim_address},
    },
    dashmap::DashMap,
    flatbuffers::FlatBufferBuilder,
    itertools::Itertools,
    solana_rpc_client_api::{
        client_error::Error,
        config::RpcSendTransactionConfig,
    },
    solana_sdk::{
        account::Account,
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction, InstructionError},
        message::{v0, VersionedMessage},
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::{Signer, SignerError},
        system_program,
        transaction::{TransactionError, VersionedTransaction},
        compute_budget::ComputeBudgetInstruction,
        system_instruction,
    },
    solana_transaction_status::TransactionStatus as TransactionConfirmationStatus,
    tokio::task::JoinHandle,
    tracing::{error, info, debug},
};

use {
    crate::types::ProgramExec,
    anyhow::Result,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending { expiry: u64 },
    Confirmed(TransactionConfirmationStatus),
}

#[async_trait]
pub trait TransactionSender {
    fn start(&mut self);
    async fn claim(
        &self,
        execution_id: &str,
        requester: Pubkey,
        execution_account: Pubkey,
        block_commitment: u64,
    ) -> Result<Signature>;
    async fn submit_proof(
        &self,
        execution_id: &str,
        requester_account: Pubkey,
        callback_exec: Option<ProgramExec>,
        proof: &[u8],
        execution_digest: &[u8],
        input_digest: &[u8],
        assumption_digest: &[u8],
        committed_outputs: &[u8],
        additional_accounts: Vec<AccountMeta>,
        exit_code_system: u32,
        exit_code_user: u32,
    ) -> Result<Signature>;
    async fn get_current_block(&self) -> Result<u64>;
    fn get_signature_status(&self, sig: &Signature) -> Option<TransactionStatus>;
    fn clear_signature_status(&self, sig: &Signature);
    async fn get_deployment_account(&self, image_id: &str) -> Result<Account>;
}

pub struct RpcTransactionSender {
    pub rpc_client: Arc<RpcClient>,
    pub bonsol_program: Pubkey,
    pub signer: Keypair,
    pub txn_status_handle: Option<JoinHandle<()>>,
    pub sigs: Arc<DashMap<Signature, TransactionStatus>>,
}

impl Signer for RpcTransactionSender {
    fn pubkey(&self) -> Pubkey {
        self.signer.pubkey()
    }

    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.signer.pubkey())
    }

    fn sign_message(&self, message: &[u8]) -> Signature {
        self.signer.sign_message(message)
    }

    fn try_sign_message(
        &self,
        message: &[u8],
    ) -> std::result::Result<Signature, solana_sdk::signer::SignerError> {
        self.signer.try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        false
    }
}

impl RpcTransactionSender {
    pub fn new(rpc_url: String, bonsol_program: Pubkey, signer: Keypair) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            signer,
            bonsol_program,
            txn_status_handle: None,
            sigs: Arc::new(DashMap::new()),
        }
    }

    async fn create_compute_budget_instructions(&self) -> Result<Vec<Instruction>> {
        // Set high compute limit for proof verification
        let compute_limit = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
        
        // Get current network fees
        let prioritization_fees = self.rpc_client
            .get_recent_prioritization_fees(&[self.signer.pubkey(), self.bonsol_program])
            .await?;
            
        let compute_price = if prioritization_fees.is_empty() {
            ComputeBudgetInstruction::set_compute_unit_price(5)
        } else {
            ComputeBudgetInstruction::set_compute_unit_price(prioritization_fees[0].prioritization_fee)
        };
        
        Ok(vec![compute_limit, compute_price])
    }

    async fn get_rent_exempt_balance(&self, data_len: usize) -> Result<u64> {
        let rent = self.rpc_client.get_minimum_balance_for_rent_exemption(data_len).await?;
        Ok(rent)
    }

    async fn create_rent_funding_instructions(
        &self,
        accounts: &[AccountMeta],
        data_lengths: &[usize],
    ) -> Result<Vec<Instruction>> {
        info!("\n🔍 Checking Rent Funding Requirements");
        debug!("Number of accounts to check: {}", accounts.len());
        debug!("Data lengths provided: {:?}", data_lengths);
        
        let mut instructions = Vec::new();
        
        for (i, account) in accounts.iter().enumerate() {
            debug!("\nAccount {} Analysis:", i);
            debug!("Address: {}", account.pubkey);
            debug!("Is Writable: {}", account.is_writable);
            
            if !account.is_writable {
                debug!("Skipping non-writable account");
                continue;
            }
            
            info!("Fetching account info for {}", account.pubkey);
            let account_info = match self.rpc_client.get_account(&account.pubkey).await {
                Ok(info) => {
                    debug!("✓ Account info retrieved");
                    debug!("Current balance: {} lamports", info.lamports);
                    debug!("Current data size: {} bytes", info.data.len());
                    info
                }
                Err(e) => {
                    debug!("Account not found (expected for new accounts): {:?}", e);
                    debug!("Proceeding with zero balance assumption");
                    solana_sdk::account::Account {
                        lamports: 0,
                        data: vec![],
                        owner: solana_sdk::system_program::ID,
                        executable: false,
                        rent_epoch: 0,
                    }
                }
            };
            
            let data_len = data_lengths.get(i).copied().unwrap_or(0);
            debug!("Required data length: {} bytes", data_len);
            
            debug!("Calculating rent-exempt balance...");
            let required_balance = match self.get_rent_exempt_balance(data_len).await {
                Ok(balance) => {
                    debug!("✓ Required balance calculated: {} lamports", balance);
                    balance
                }
                Err(e) => {
                    error!("Failed to calculate rent-exempt balance: {:?}", e);
                    return Err(e.into());
                }
            };
            
            if account_info.lamports < required_balance {
                let transfer_amount = required_balance.saturating_sub(account_info.lamports);
                info!("💰 Account {} requires funding:", account.pubkey);
                debug!("Current balance: {} lamports", account_info.lamports);
                debug!("Required balance: {} lamports", required_balance);
                debug!("Transfer amount: {} lamports", transfer_amount);
                
                debug!("Creating transfer instruction...");
                instructions.push(
                    system_instruction::transfer(
                        &self.signer.pubkey(),
                        &account.pubkey,
                        transfer_amount,
                    )
                );
                debug!("✓ Transfer instruction created");
            } else {
                debug!("✓ Account {} is sufficiently funded", account.pubkey);
                debug!("Current: {} lamports", account_info.lamports);
                debug!("Required: {} lamports", required_balance);
            }
        }
        
        info!("\n📋 Rent Funding Summary: {} instructions created", instructions.len());
        if !instructions.is_empty() {
            debug!("Transfer Instructions:");
            for (i, ix) in instructions.iter().enumerate() {
                debug!("Instruction {}:", i);
                debug!("  From: {}", self.signer.pubkey());
                debug!("  To: {}", ix.accounts[1].pubkey);
                // The amount is the last 8 bytes of the instruction data
                let amount = if ix.data.len() >= 8 {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&ix.data[ix.data.len()-8..]);
                    u64::from_le_bytes(bytes)
                } else {
                    0
                };
                debug!("  Amount: {} lamports", amount);
            }
        }
        
        Ok(instructions)
    }
}

#[async_trait]
impl TransactionSender for RpcTransactionSender {
    fn get_signature_status(&self, sig: &Signature) -> Option<TransactionStatus> {
        self.sigs.get(sig).map(|status| status.value().to_owned())
    }

    fn clear_signature_status(&self, sig: &Signature) {
        self.sigs.remove(sig);
    }

    async fn claim(
        &self,
        execution_id: &str,
        requester: Pubkey,
        execution_account: Pubkey,
        block_commitment: u64,
    ) -> Result<Signature> {
        let (execution_claim_account, _) = execution_claim_address(execution_account.as_ref());
        let accounts = vec![
            AccountMeta::new(execution_account, false),
            AccountMeta::new_readonly(requester, false),
            AccountMeta::new(execution_claim_account, false),
            AccountMeta::new(self.signer.pubkey(), true),
            AccountMeta::new(self.signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];
        let mut fbb = FlatBufferBuilder::new();
        let eid = fbb.create_string(execution_id);
        let stat = ClaimV1::create(
            &mut fbb,
            &ClaimV1Args {
                block_commitment,
                execution_id: Some(eid),
            },
        );
        fbb.finish(stat, None);
        let statbytes = fbb.finished_data();
        let mut fbb2 = FlatBufferBuilder::new();
        let off = fbb2.create_vector(statbytes);
        let root = ChannelInstruction::create(
            &mut fbb2,
            &ChannelInstructionArgs {
                ix_type: ChannelInstructionIxType::ClaimV1,
                claim_v1: Some(off),
                ..Default::default()
            },
        );
        fbb2.finish(root, None);
        let ix_data = fbb2.finished_data();
        let instruction = Instruction::new_with_bytes(self.bonsol_program, ix_data, accounts);
        
        // Add compute budget instructions
        let mut instructions = self.create_compute_budget_instructions().await?;
        instructions.push(instruction);

        let (blockhash_req, last_valid) = self
            .rpc_client
            .get_latest_blockhash_with_commitment(self.rpc_client.commitment())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {:?}", e))?;

        let msg =
            v0::Message::try_compile(&self.signer.pubkey(), &instructions, &[], blockhash_req)?;
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&self.signer])?;
        let sig = self
            .rpc_client
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;
        self.sigs
            .insert(sig, TransactionStatus::Pending { expiry: last_valid });
        Ok(sig)
    }

    async fn submit_proof(
        &self,
        execution_id: &str,
        requester_account: Pubkey,
        callback_exec: Option<ProgramExec>,
        proof: &[u8],
        execution_digest: &[u8],
        input_digest: &[u8],
        assumption_digest: &[u8],
        committed_outputs: &[u8],
        additional_accounts: Vec<AccountMeta>,
        exit_code_system: u32,
        exit_code_user: u32,
    ) -> Result<Signature> {
        debug!("📦 Proof Data Analysis:");
        debug!("Proof length: {} bytes", proof.len());
        debug!("Execution digest length: {} bytes", execution_digest.len());
        debug!("Input digest length: {} bytes", input_digest.len());
        debug!("Assumption digest length: {} bytes", assumption_digest.len());
        debug!("Committed outputs length: {} bytes", committed_outputs.len());
        debug!("First 32 bytes of committed outputs: {:02x?}", &committed_outputs[..32.min(committed_outputs.len())]);
        
        if let Some(marker_pos) = committed_outputs.iter().position(|&x| x == 0xaa) {
            debug!("Found marker 0xAA at position {}", marker_pos);
            if committed_outputs.len() >= marker_pos + 7 {
                let line_values = &committed_outputs[marker_pos + 1..marker_pos + 7];
                debug!("Line values: {:02x?}", line_values);
                debug!("Line values valid: {}", line_values.iter().all(|&x| (6..=9).contains(&x)));
                
                if committed_outputs.len() > marker_pos + 7 {
                    let ascii_art = String::from_utf8_lossy(&committed_outputs[marker_pos + 7..]);
                    debug!("ASCII art length: {} bytes", ascii_art.len());
                    debug!("ASCII art content:\n{}", ascii_art);
                }
            } else {
                debug!("❌ Insufficient data after marker: {} bytes", committed_outputs.len() - marker_pos);
            }
        } else {
            debug!("❌ No 0xAA marker found in committed outputs");
            debug!("Full committed outputs: {:02x?}", committed_outputs);
        }

        // Create FlatBuffer for status
        debug!("\n🔧 Building Status FlatBuffer");
        let mut builder = FlatBufferBuilder::new();
        
        // Create vectors for byte arrays
        let proof_vec = builder.create_vector(proof);
        let execution_digest_vec = builder.create_vector(execution_digest);
        let input_digest_vec = builder.create_vector(input_digest);
        let assumption_digest_vec = builder.create_vector(assumption_digest);
        let committed_outputs_vec = builder.create_vector(committed_outputs);
        
        debug!("Created FlatBuffer vectors:");
        debug!("- Proof: {} bytes", proof.len());
        debug!("- Execution digest: {} bytes", execution_digest.len());
        debug!("- Input digest: {} bytes", input_digest.len());
        debug!("- Assumption digest: {} bytes", assumption_digest.len());
        debug!("- Committed outputs: {} bytes", committed_outputs.len());

        // Create execution_id string
        let execution_id_str = builder.create_string(execution_id);

        let status = StatusV1Args {
            execution_id: Some(execution_id_str),
            status: StatusTypes::Completed,
            exit_code_system,
            exit_code_user,
            proof: Some(proof_vec),
            execution_digest: Some(execution_digest_vec),
            input_digest: Some(input_digest_vec),
            assumption_digest: Some(assumption_digest_vec),
            committed_outputs: Some(committed_outputs_vec),
        };
        
        let status_offset = StatusV1::create(&mut builder, &status);
        builder.finish(status_offset, None);
        let status_data = builder.finished_data();
        
        debug!("\n📋 Final Status Data:");
        debug!("Total size: {} bytes", status_data.len());
        debug!("First 32 bytes: {:02x?}", &status_data[..32.min(status_data.len())]);

        // Get program ID and create instruction data
        let (program_id, instruction_prefix) = if let Some(ref pe) = callback_exec {
            debug!("\n🔑 Using callback configuration:");
            debug!("Program ID: {}", pe.program_id);
            debug!("Instruction prefix: {:?}", pe.instruction_prefix);
            (pe.program_id, pe.instruction_prefix.clone())
        } else {
            debug!("\n🔑 Using default Bonsol program");
            (self.bonsol_program, vec![])
        };

        // Create instruction data
        let mut instruction_data = instruction_prefix.clone();
        instruction_data.extend_from_slice(status_data);
        
        debug!("\n📝 Final Instruction Data:");
        debug!("Prefix length: {} bytes", instruction_prefix.len());
        debug!("Total length: {} bytes", instruction_data.len());
        debug!("First 32 bytes: {:02x?}", &instruction_data[..32.min(instruction_data.len())]);

        // Get accounts
        let execution_account = execution_address(&requester_account, execution_id.as_bytes()).0;
        
        debug!("\n👥 Account Configuration:");
        debug!("Execution account: {}", execution_account);
        debug!("Additional accounts: {}", additional_accounts.len());
        for (i, acc) in additional_accounts.iter().enumerate() {
            debug!("Account {}: {} (signer: {}, writable: {})", 
                i, acc.pubkey, acc.is_signer, acc.is_writable);
        }

        // Create compute budget instructions
        let compute_ixs = self.create_compute_budget_instructions().await?;
        debug!("\n💻 Compute Budget Instructions: {}", compute_ixs.len());
        
        // Create rent funding instructions
        let mut all_accounts = vec![
            AccountMeta::new(self.signer.pubkey(), true),
            AccountMeta::new(execution_account, false),
        ];
        all_accounts.extend(additional_accounts.iter().cloned());
        
        let rent_ixs = self.create_rent_funding_instructions(&all_accounts, &[0]).await?;
        debug!("💰 Rent Funding Instructions: {}", rent_ixs.len());

        // Build main instruction
        let instruction = Instruction {
            program_id,
            accounts: all_accounts,
            data: instruction_data,
        };

        // Combine all instructions
        let mut instructions = Vec::new();
        instructions.extend(compute_ixs);
        instructions.extend(rent_ixs);
        instructions.push(instruction);
        
        debug!("\n🚀 Final Transaction:");
        debug!("Total instructions: {}", instructions.len());
        debug!("Sending transaction...");

        // Send transaction
        let (blockhash, last_valid_block_height) = self
            .rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;

        let sig = self
            .rpc_client
            .send_transaction_with_config(
                &VersionedTransaction::try_new(
                    VersionedMessage::V0(v0::Message::try_compile(
                        &self.signer.pubkey(),
                        &instructions,
                        &[],
                        blockhash,
                    )?),
                    &[&self.signer],
                )?,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: Some(CommitmentConfig::processed().commitment),
                    max_retries: Some(5),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                error!("Transaction failed: {:?}", e);
                if let Some(code) = extract_custom_error(&e) {
                    error!("Custom program error code: 0x{:x}", code);
                }
                anyhow::anyhow!("Failed to send transaction: {:?}", e)
            })?;
            
        info!("Transaction sent successfully: {}", sig);
        self.sigs
            .insert(sig, TransactionStatus::Pending { expiry: last_valid_block_height });
        Ok(sig)
    }

    fn start(&mut self) {
        let sigs_ref = self.sigs.clone();
        let rpc_client = self.rpc_client.clone();
        self.txn_status_handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let current_block_height = rpc_client
                    .get_block_height_with_commitment(rpc_client.commitment())
                    .await;

                if let Ok(current_block_height) = current_block_height {
                    sigs_ref.retain(|k, v| {
                        if let TransactionStatus::Pending { expiry } = v {
                            if *expiry < current_block_height {
                                info!("Transaction expired {}", k);
                                return false;
                            }
                        }
                        true
                    });
                    let all_sigs = sigs_ref.iter().map(|x| *x.key()).collect_vec();
                    let statuses = rpc_client.get_signature_statuses(&all_sigs).await;
                    if let Ok(statuses) = statuses {
                        for sig in all_sigs.into_iter().zip(statuses.value.into_iter()) {
                            if let Some(status) = sig.1 {
                                sigs_ref.insert(sig.0, TransactionStatus::Confirmed(status));
                            }
                        }
                    }
                } else {
                    error!("Failed to get block height");
                }
            }
        }));
    }

    async fn get_current_block(&self) -> Result<u64> {
        self.rpc_client
            .get_block_height()
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    async fn get_deployment_account(&self, image_id: &str) -> Result<Account> {
        let (deployment_account, _) = deployment_address(image_id);
        self.rpc_client
            .get_account(&deployment_account)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))
    }
}

fn extract_custom_error(error: &Error) -> Option<u32> {
    if let Error { kind: solana_rpc_client_api::client_error::ErrorKind::TransactionError(
        TransactionError::InstructionError(_, InstructionError::Custom(code))
    ), .. } = error {
        Some(*code)
    } else {
        None
    }
}
