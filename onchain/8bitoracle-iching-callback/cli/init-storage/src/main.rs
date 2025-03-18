use {
    bitoracle_iching_callback::{id, CallbackInstruction},
    borsh::BorshSerialize,
    solana_client::rpc_client::RpcClient,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program::ID as SYS_ID,
    },
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::read_keypair_file,
        signer::Signer,
        transaction::Transaction,
    },
    std::{env, str::FromStr},
};

fn print_usage() {
    eprintln!("Usage: init-storage --storage-address <ADDRESS> --payer <ADDRESS> --keypair <KEYPAIR_PATH> [--url <RPC_URL>]");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse command line arguments
    let mut storage_address = None;
    let mut payer = None;
    let mut keypair_path = None;
    let mut url = Some(String::from("http://127.0.0.1:8899")); // Default to localhost
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--storage-address" => {
                i += 1;
                if i < args.len() {
                    storage_address = Some(args[i].clone());
                }
            }
            "--payer" => {
                i += 1;
                if i < args.len() {
                    payer = Some(args[i].clone());
                }
            }
            "--keypair" => {
                i += 1;
                if i < args.len() {
                    keypair_path = Some(args[i].clone());
                }
            }
            "--url" => {
                i += 1;
                if i < args.len() {
                    url = Some(args[i].clone());
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
            }
        }
        i += 1;
    }
    
    // Validate required arguments
    let storage_address = storage_address.unwrap_or_else(|| {
        eprintln!("Missing required argument: --storage-address");
        print_usage();
        unreachable!();
    });
    
    let payer = payer.unwrap_or_else(|| {
        eprintln!("Missing required argument: --payer");
        print_usage();
        unreachable!();
    });

    let keypair_path = keypair_path.unwrap_or_else(|| {
        eprintln!("Missing required argument: --keypair");
        print_usage();
        unreachable!();
    });

    let url = url.unwrap();

    // Parse addresses
    let storage_pubkey = Pubkey::from_str(&storage_address)
        .unwrap_or_else(|_| {
            eprintln!("Invalid storage address");
            std::process::exit(1);
        });
    
    let payer_pubkey = Pubkey::from_str(&payer)
        .unwrap_or_else(|_| {
            eprintln!("Invalid payer address");
            std::process::exit(1);
        });

    // Load payer keypair from file
    let payer_keypair = read_keypair_file(&keypair_path)
        .unwrap_or_else(|err| {
            eprintln!("Failed to read keypair file: {}", err);
            std::process::exit(1);
        });

    // Ensure the loaded keypair matches the specified payer
    if payer_keypair.pubkey() != payer_pubkey {
        eprintln!("Warning: Keypair pubkey {} does not match specified payer {}", 
                 payer_keypair.pubkey(), payer_pubkey);
        eprintln!("Using keypair pubkey as payer");
    }

    // Initialize RPC client
    println!("Connecting to Solana node at {}", url);
    let client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

    // Check if the account already exists
    match client.get_account(&storage_pubkey) {
        Ok(_) => {
            println!("Storage account {} already exists", storage_pubkey);
            std::process::exit(0);
        },
        Err(_) => {
            println!("Storage account does not exist, proceeding with initialization");
        }
    }

    // Create Initialize instruction data
    let init_data = CallbackInstruction::Initialize;
    let mut init_data_bytes = Vec::new();
    init_data.serialize(&mut init_data_bytes).unwrap_or_else(|err| {
        eprintln!("Failed to serialize initialization data: {}", err);
        std::process::exit(1);
    });

    // Create the instruction
    let instruction = Instruction::new_with_bytes(
        id(), // Use the program's ID
        &init_data_bytes,
        vec![
            AccountMeta::new(payer_keypair.pubkey(), true),  // Payer (signer)
            AccountMeta::new(storage_pubkey, false),         // Storage PDA (writable, not signer)
            AccountMeta::new_readonly(SYS_ID, false),        // System program
        ],
    );

    // Get recent blockhash
    println!("Getting recent blockhash...");
    let recent_blockhash = client.get_latest_blockhash()
        .unwrap_or_else(|err| {
            eprintln!("Failed to get recent blockhash: {}", err);
            std::process::exit(1);
        });

    // Create and sign transaction
    println!("Creating transaction...");
    let mut transaction = Transaction::new_with_payer(
        &[instruction],
        Some(&payer_keypair.pubkey()),
    );
    
    transaction.sign(&[&payer_keypair], recent_blockhash);

    // Send and confirm transaction
    println!("Sending transaction to initialize storage account...");
    match client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            println!("Transaction successful!");
            println!("Signature: {}", signature);
            println!("Storage account {} successfully initialized", storage_pubkey);
        },
        Err(err) => {
            eprintln!("Failed to send transaction: {}", err);
            std::process::exit(1);
        }
    }
} 