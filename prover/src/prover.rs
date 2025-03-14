use std::rc::Rc;

use anyhow::Result;
use bonsol_schema::ProgramInputType;
use risc0_binfmt::MemoryImage;
use risc0_zkvm::{get_prover_server, ExecutorEnv, ExecutorImpl, ProverOpts, ProverServer, Receipt};
use tracing::{info, error};

use crate::input_resolver::ProgramInput;

/// Creates a new risc0 executor environment from the provided inputs, it handles setting up the execution env in the same way across types of provers.
pub async fn new_risc0_exec_env(
    image: MemoryImage,
    sorted_inputs: Vec<ProgramInput>,
) -> Result<ExecutorImpl<'static>> {
    info!("Creating new RISC0 executor environment");
    info!("Memory image size: {} pages", image.pages.len());
    info!("Number of inputs: {}", sorted_inputs.len());
    
    let mut env_builder = ExecutorEnv::builder();
    
    // Create a copy of sorted_inputs for logging
    let input_count = sorted_inputs.len();
    let sorted_inputs = sorted_inputs.into_iter().collect::<Vec<_>>();
    
    for (i, input) in sorted_inputs.iter().enumerate() {
        info!("Processing input {}/{}", i + 1, input_count);
        match input {
            ProgramInput::Resolved(ri) => {
                if ri.input_type == ProgramInputType::PublicProof {
                    info!("Adding proof assumption for input {}", i);
                    let receipt: Receipt = match bincode::deserialize(&ri.data) {
                        Ok(r) => {
                            info!("Successfully deserialized receipt for input {}", i);
                            r
                        },
                        Err(e) => {
                            error!("Failed to deserialize receipt for input {}: {}", i, e);
                            return Err(anyhow::anyhow!("Receipt deserialization failed: {}", e));
                        }
                    };
                    env_builder.add_assumption(receipt);
                } else {
                    info!("Writing {} bytes for input {}", ri.data.len(), i);
                    env_builder.write_slice(&ri.data);
                }
            }
            _ => {
                error!("Invalid input type for input {}", i);
                return Err(anyhow::anyhow!("Invalid input type"));
            }
        }
    }
    
    info!("Building executor environment");
    let env = env_builder.build()?;
    info!("Creating executor implementation");
    let executor = ExecutorImpl::new(env, image)?;
    info!("Executor environment created successfully");
    Ok(executor)
}

/// Gets the default r0 prover for this application
/// Since the cli and the node both produce proofs there is a need for a central prover configuration.
/// Note: This returns Rc since the prover should only be used in blocking contexts with tokio::task::spawn_blocking
pub fn get_risc0_prover() -> Result<Rc<dyn ProverServer>> {
    info!("Initializing RISC0 prover");
    info!("RISC0_DEV_MODE: {}", option_env!("RISC0_DEV_MODE").is_some());
    
    let opts = ProverOpts::default();
    info!("Using prover options:");
    info!("  - Prove guest errors: {}", opts.prove_guest_errors);
    
    let prover = get_prover_server(&opts)?;
    info!("Prover initialized successfully");
    Ok(prover)
}
