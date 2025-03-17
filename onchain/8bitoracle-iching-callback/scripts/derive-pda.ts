import { PublicKey } from "@solana/web3.js";
import { createHash } from "crypto";

// Constants from our Rust program
const HEXAGRAM_SEED = Buffer.from("hexagram");
const EXECUTION_SEED_PREFIX = Buffer.from("execution");
const DEPLOYMENT_SEED_PREFIX = Buffer.from("deployment");

// Function to convert hex string to Uint8Array
function hexToBytes(hex: string): Uint8Array {
  // Remove 0x prefix if present
  hex = hex.replace(/^0x/, '');
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}

// Function to derive the execution account PDA
async function deriveExecutionAddress(
  requester: PublicKey,
  executionId: string,
  bonsolProgramId: PublicKey
): Promise<[PublicKey, number]> {
  // Debug logging to stderr
  console.error("============ Execution PDA Derivation Debug ============");
  console.error("EXECUTION_SEED_PREFIX:", {
    length: EXECUTION_SEED_PREFIX.length,
    bytes: [...EXECUTION_SEED_PREFIX],
    utf8: EXECUTION_SEED_PREFIX.toString("utf8"),
  });
  console.error("Requester:", {
    pubkey: requester.toBase58(),
    bytes: [...requester.toBuffer()],
    length: requester.toBuffer().length,
  });
  console.error("Execution ID:", {
    value: executionId,
    bytes: [...hexToBytes(executionId)],
    length: hexToBytes(executionId).length,
  });
  console.error("Bonsol Program ID:", bonsolProgramId.toBase58());

  const seeds = [
    EXECUTION_SEED_PREFIX,
    requester.toBuffer(),
    hexToBytes(executionId),
  ];

  console.error(
    "Seeds array:",
    seeds.map((seed, i) => ({
      index: i,
      length: seed.length,
      bytes: [...seed],
    }))
  );

  const [pda, bump] = await PublicKey.findProgramAddress(
    seeds,
    bonsolProgramId
  );
  console.error("Derived Execution PDA:", {
    pubkey: pda.toBase58(),
    bump,
  });
  console.error("================================================");

  return [pda, bump];
}

// Function to derive the PDA for hexagram storage
export function deriveHexagramAddress(
  payer: PublicKey,
  callbackProgram: PublicKey
): [PublicKey, number] {
  // Debug logging to stderr
  console.error("============ Hexagram PDA Derivation Debug ============");
  console.error("HEXAGRAM_SEED:", {
    length: HEXAGRAM_SEED.length,
    bytes: [...HEXAGRAM_SEED],
    utf8: HEXAGRAM_SEED.toString("utf8"),
  });
  console.error("Payer:", {
    pubkey: payer.toBase58(),
    bytes: [...payer.toBuffer()],
    length: payer.toBuffer().length,
  });
  console.error("Callback Program:", callbackProgram.toBase58());

  const [pda, bump] = PublicKey.findProgramAddressSync(
    [
      HEXAGRAM_SEED,
      payer.toBuffer(),
    ],
    callbackProgram
  );

  console.error("Derived Hexagram PDA:", {
    pubkey: pda.toBase58(),
    bump,
  });
  console.error("============================================");

  return [pda, bump];
}

// Function to derive the deployment account PDA
async function deriveDeploymentAddress(
  imageId: string,
  bonsolProgramId: PublicKey
): Promise<[PublicKey, number]> {
  // Debug logging to stderr
  console.error("============ Deployment PDA Derivation Debug ============");
  console.error("DEPLOYMENT_SEED_PREFIX:", {
    length: DEPLOYMENT_SEED_PREFIX.length,
    bytes: [...DEPLOYMENT_SEED_PREFIX],
    utf8: DEPLOYMENT_SEED_PREFIX.toString("utf8"),
  });
  
  // Convert image ID to bytes
  const imageIdBytes = hexToBytes(imageId);
  console.error("Image ID:", {
    imageId,
    bytes: [...imageIdBytes],
    length: imageIdBytes.length,
  });

  const seeds = [DEPLOYMENT_SEED_PREFIX, imageIdBytes];

  console.error(
    "Seeds array:",
    seeds.map((seed, i) => ({
      index: i,
      length: seed.length,
      bytes: [...seed],
    }))
  );

  const [pda, bump] = await PublicKey.findProgramAddress(seeds, bonsolProgramId);
  console.error("Derived Deployment PDA:", {
    pubkey: pda.toBase58(),
    bump,
  });
  console.error("================================================");

  return [pda, bump];
}

// Main function
async function main() {
  // Get program ID, requester, execution ID from command line args
  const args = process.argv.slice(2);
  if (args.length !== 5) {
    console.error(
      "Usage: ts-node derive-pda.ts <callback_program_id> <payer_pubkey> <bonsol_program_id> <execution_id> <image_id>"
    );
    process.exit(1);
  }

  const [callbackProgramId, payerStr, bonsolProgramIdStr, executionId, imageId] =
    args;
  console.error("\n============ PDA Derivation Started ============");
  console.error("Command line arguments:");
  console.error("- Callback Program ID:", callbackProgramId);
  console.error("- Payer:", payerStr);
  console.error("- Bonsol Program ID:", bonsolProgramIdStr);
  console.error("- Execution ID:", executionId);
  console.error("- Image ID:", imageId);

  try {
    const callbackProgram = new PublicKey(callbackProgramId);
    const payer = new PublicKey(payerStr);
    const bonsolProgram = new PublicKey(bonsolProgramIdStr);

    // First derive the execution account PDA using Bonsol program
    console.error("\nDeriving execution PDA...");
    const [executionPda, executionBump] = await deriveExecutionAddress(
      payer,
      executionId,
      bonsolProgram
    );
    console.error("Execution PDA derived successfully:", {
      pda: executionPda.toBase58(),
      bump: executionBump,
    });

    // Then derive the hexagram storage PDA using payer
    console.error("\nDeriving hexagram PDA...");
    const [hexagramPda, hexagramBump] = deriveHexagramAddress(
      payer,
      callbackProgram
    );
    console.error("Hexagram PDA derived successfully:", {
      pda: hexagramPda.toBase58(),
      bump: hexagramBump,
    });

    // Finally derive the deployment account PDA
    console.error("\nDeriving deployment PDA...");
    const [deploymentPda, deploymentBump] = await deriveDeploymentAddress(
      imageId,
      bonsolProgram
    );
    console.error("Deployment PDA derived successfully:", {
      pda: deploymentPda.toBase58(),
      bump: deploymentBump,
    });

    // Output summary to stderr
    console.error("\n============ Final Results ============");
    console.error("Execution PDA:", executionPda.toBase58());
    console.error("Hexagram PDA:", hexagramPda.toBase58());
    console.error("Deployment PDA:", deploymentPda.toBase58());
    console.error("=====================================\n");

    // Output ONLY the PDAs to stdout for machine consumption
    console.log(`${executionPda.toBase58()}\n${hexagramPda.toBase58()}\n${deploymentPda.toBase58()}`);
  } catch (error) {
    console.error("\nError during PDA derivation:", error);
    process.exit(1);
  }
}

main().catch(console.error);
