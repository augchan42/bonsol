import { PublicKey } from "@solana/web3.js";
import { createHash } from "crypto";

// Constants from our Rust program
const HEXAGRAM_SEED_PREFIX = Buffer.from("8bitoracle-hexagram");
const HEXAGRAM_SEED_VERSION = Buffer.from("v1");
const EXECUTION_SEED_PREFIX = Buffer.from("execution");
const DEPLOYMENT_SEED_PREFIX = Buffer.from("deployment");

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
    bytes: [...Buffer.from(executionId)],
    length: Buffer.from(executionId).length,
  });
  console.error("Bonsol Program ID:", bonsolProgramId.toBase58());

  const seeds = [
    EXECUTION_SEED_PREFIX,
    requester.toBuffer(),
    Buffer.from(executionId),
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
  executionPda: PublicKey,
  callbackProgram: PublicKey
): [PublicKey, number] {
  // Debug logging to stderr
  console.error("============ PDA Derivation Debug ============");
  console.error("HEXAGRAM_SEED_PREFIX:", HEXAGRAM_SEED_PREFIX);
  console.error("HEXAGRAM_SEED_VERSION:", HEXAGRAM_SEED_VERSION);
  console.error("Execution PDA:", executionPda.toBase58());
  console.error("Callback Program:", callbackProgram.toBase58());

  const [pda, bump] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(HEXAGRAM_SEED_PREFIX),
      Buffer.from(HEXAGRAM_SEED_VERSION),
      executionPda.toBuffer(),
    ],
    callbackProgram
  );

  console.error("Derived PDA:", pda.toBase58());
  console.error("Bump:", bump);
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
  
  // Hash the image ID
  const imageIdHash = createHash("sha256").update(imageId).digest();
  console.error("Image ID Hash:", {
    imageId,
    hash: imageIdHash.toString("hex"),
    length: imageIdHash.length,
  });

  const seeds = [DEPLOYMENT_SEED_PREFIX, imageIdHash];

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
      "Usage: ts-node derive-pda.ts <callback_program_id> <requester_pubkey> <bonsol_program_id> <execution_id> <image_id>"
    );
    process.exit(1);
  }

  const [callbackProgramId, requesterStr, bonsolProgramIdStr, executionId, imageId] =
    args;
  console.error("\n============ PDA Derivation Started ============");
  console.error("Command line arguments:");
  console.error("- Callback Program ID:", callbackProgramId);
  console.error("- Requester:", requesterStr);
  console.error("- Bonsol Program ID:", bonsolProgramIdStr);
  console.error("- Execution ID:", executionId);
  console.error("- Image ID:", imageId);

  try {
    const callbackProgram = new PublicKey(callbackProgramId);
    const requester = new PublicKey(requesterStr);
    const bonsolProgram = new PublicKey(bonsolProgramIdStr);

    // First derive the execution account PDA using Bonsol program
    console.error("\nDeriving execution PDA...");
    const [executionPda, executionBump] = await deriveExecutionAddress(
      requester,
      executionId,
      bonsolProgram
    );
    console.error("Execution PDA derived successfully:", {
      pda: executionPda.toBase58(),
      bump: executionBump,
    });

    // Then derive the hexagram storage PDA using execution PDA
    console.error("\nDeriving hexagram PDA...");
    const [hexagramPda, hexagramBump] = deriveHexagramAddress(
      executionPda,
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
