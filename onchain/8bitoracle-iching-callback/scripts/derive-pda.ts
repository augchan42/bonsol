import { PublicKey } from '@solana/web3.js';

// Constants from our Rust program
const HEXAGRAM_SEED_PREFIX = Buffer.from('8bitoracle-hexagram');
const HEXAGRAM_SEED_VERSION = Buffer.from('v1');

// Function to derive the PDA for hexagram storage
async function deriveHexagramAddress(
  executionAccount: PublicKey,
  programId: PublicKey,
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [
      HEXAGRAM_SEED_PREFIX,
      HEXAGRAM_SEED_VERSION,
      executionAccount.toBuffer(),
    ],
    programId,
  );
}

// Main function
async function main() {
  // Get program ID and execution account from command line args
  const args = process.argv.slice(2);
  if (args.length !== 2) {
    console.error('Usage: ts-node derive-pda.ts <program_id> <execution_account>');
    process.exit(1);
  }

  const [programIdStr, executionAccountStr] = args;

  try {
    const programId = new PublicKey(programIdStr);
    const executionAccount = new PublicKey(executionAccountStr);

    const [pda, bump] = await deriveHexagramAddress(executionAccount, programId);
    
    console.log('Derived PDA for hexagram storage:');
    console.log('PDA:', pda.toBase58());
    console.log('Bump seed:', bump);
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main().catch(console.error); 