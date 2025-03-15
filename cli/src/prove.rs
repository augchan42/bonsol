use crate::common::{proof_get_inputs, ZkProgramManifest};
use anyhow::{anyhow, Result};
use bonsol_prover::image::Image;
use bonsol_prover::prover::{get_risc0_prover, new_risc0_exec_env};
use bonsol_sdk::BonsolClient;
use bytes::Bytes;
use log::{debug, error, info};
use risc0_zkvm::VerifierContext;
use std::fs::{read, File};
use std::io::Write;
use std::path::Path;

pub async fn prove(
    sdk: &BonsolClient,
    execution_id: String,
    manifest_path: Option<String>,
    program_id: Option<String>,
    input_file: Option<String>,
    output_location: Option<String>,
    stdin: Option<String>,
) -> Result<()> {
    info!("Starting proof generation for execution ID: {}", execution_id);
    debug!("Configuration:");
    debug!("  Manifest path: {:?}", manifest_path);
    debug!("  Program ID: {:?}", program_id);
    debug!("  Input file: {:?}", input_file);
    debug!("  Output location: {:?}", output_location);
    debug!("  Stdin: {:?}", stdin);

    let pwd = std::env::current_dir()?;
    debug!("Current working directory: {:?}", pwd);

    let image_bytes = match (&program_id, manifest_path) {
        (Some(i), None) => {
            info!("Downloading program with ID: {}", i);
            let bytes: Bytes = sdk.download_program(i).await?;
            debug!("Downloaded program size: {} bytes", bytes.len());
            Ok(bytes)
        }
        (None, Some(m)) => {
            info!("Loading program from manifest: {}", m);
            let manifest_path = Path::new(&m);
            let manifest_file = if manifest_path.is_relative() {
                debug!("Using relative manifest path: {:?}", manifest_path);
                File::open(pwd.join(manifest_path))?
            } else {
                debug!("Using absolute manifest path: {:?}", manifest_path);
                File::open(manifest_path)?
            };
            let manifest: ZkProgramManifest = serde_json::from_reader(manifest_file)?;
            debug!("Loaded manifest: {:?}", manifest);
            
            let binary_path = Path::new(&manifest.binary_path);
            debug!("Reading binary from: {:?}", binary_path);
            let bytes = read(binary_path).map_err(|e| {
                error!("Failed to read binary: {}", e);
                anyhow!("Failed to read binary in manifest file")
            })?;
            debug!("Read binary size: {} bytes", bytes.len());
            Ok(Bytes::from(bytes))
        }
        _ => {
            error!("Neither program ID nor manifest path provided");
            Err(anyhow!("Please provide a program id or a manifest path"))
        }
    }?;

    let ext = Path::new(&execution_id).with_extension("bin");
    let output_binary_path = output_location
        .map(|o| Path::new(&o).join(&ext))
        .unwrap_or(ext);
    debug!("Output binary path: {:?}", output_binary_path);

    info!("Creating image from bytes...");
    let image = Image::from_bytes(image_bytes)?;
    debug!("Image ID: {}", image.id);
    debug!("Image size: {} bytes", image.size);

    info!("Getting memory image...");
    let memory_image = image.get_memory_image()?;
    debug!("Memory image size: {} pages", memory_image.pages.len());

    info!("Getting program inputs...");
    let program_inputs = proof_get_inputs(input_file, stdin)?;
    debug!("Number of program inputs: {}", program_inputs.len());
    
    // Create executor environment and run session
    info!("Creating executor environment...");
    let mut exec = new_risc0_exec_env(memory_image, program_inputs).await?;
    
    info!("Running executor session...");
    let session = exec.run()?;
    debug!("Session completed successfully");
    
    info!("Getting RISC0 prover...");
    let prover = get_risc0_prover()?;
    let ctx = VerifierContext::default();
    
    info!("Proving session...");
    let info = prover.prove_session(&ctx, &session)?;
    debug!("Proof generation successful");
    
    info!("Writing proof to file: {:?}", output_binary_path);
    let mut output_file = File::create(&output_binary_path)?;
    let serialized = bincode::serialize(&info.receipt)?;
    debug!("Serialized proof size: {} bytes", serialized.len());
    output_file.write_all(&serialized)?;
    
    info!("Proof generation completed successfully!");
    Ok(())
}
