mod types;
mod utils;

use risc0_zkvm::{
    guest::{env, sha::Impl},
    sha::Sha256,
};

use types::{HexagramGeneration, LineValue};
use utils::generate_line_value;

// Constants for dev mode
const DEV_MODE_MARKER: u8 = 0xAA;

fn line_to_ascii(line: LineValue) -> String {
    match line {
        LineValue::OldYin => "---x---",    // yin changing into yang (7 chars)
        LineValue::YoungYin => "--- ---",   // yin, unchanging (7 chars)
        LineValue::OldYang => "---o---",    // yang changing into yin (7 chars)
        LineValue::YoungYang => "-------",  // yang, unchanging (7 chars)
    }.to_string()
}

fn hexagram_to_ascii(hexagram: &HexagramGeneration) -> String {
    let mut ascii_art = String::with_capacity(47); // 6 lines * 7 chars + 5 newlines
    
    // Build ASCII art representation from bottom to top (lines[0] is bottom)
    for (i, &line) in hexagram.lines.iter().enumerate() {
        env::log(&format!("Converting line {} ({:?}) to ASCII", i, line));
        let line_ascii = line_to_ascii(line);
        env::log(&format!("Line {} ASCII: '{}' (len={})", i, line_ascii, line_ascii.len()));
        env::log(&format!("Line {} bytes: {:02x?}", i, line_ascii.as_bytes()));
        
        // Add line to the beginning of the string (top lines first)
        if i > 0 {
            ascii_art.insert_str(0, "\n");
            env::log(&format!("Added newline, current length: {}", ascii_art.len()));
        }
        ascii_art.insert_str(0, &line_ascii);
        env::log(&format!("Added line, current length: {}", ascii_art.len()));
    }
    
    env::log(&format!("Final ASCII art:\n{}", ascii_art));
    env::log(&format!("ASCII art length: {} bytes", ascii_art.len()));
    env::log(&format!("ASCII art bytes: {:02x?}", ascii_art.as_bytes()));
    ascii_art
}

fn main() {
    env::log("Starting I Ching hexagram generation...");
    
    // Check if we're in dev mode
    let is_dev_mode = option_env!("RISC0_DEV_MODE").is_some();
    if is_dev_mode {
        env::log("Running in dev mode (affects proof verification only)");
    }
    
    // Read the random seed
    let mut random_seed = [0u8; 32];
    env::read_slice(&mut random_seed);
    env::log(&format!("Received random seed ({}): {:02x?}", random_seed.len(), random_seed));
    
    // Generate hexagram using input seed
    let hexagram = generate_hexagram(&random_seed);
    env::log(&format!("Generated hexagram with lines: {:#?}", hexagram.lines));
    env::log(&format!("Line values (as u8): {:?}", hexagram.lines.iter().map(|&l| l as u8).collect::<Vec<_>>()));
    env::log(&format!("Line values (raw): {:?}", hexagram.lines.iter().map(|&l| match l {
        LineValue::OldYin => "OldYin (6)",
        LineValue::YoungYang => "YoungYang (7)",
        LineValue::YoungYin => "YoungYin (8)",
        LineValue::OldYang => "OldYang (9)",
    }).collect::<Vec<_>>()));
    
    // Hash of random seed
    let seed_digest = Impl::hash_bytes(&random_seed);
    let digest_bytes = seed_digest.as_bytes();
    env::log(&format!("Generated seed digest ({} bytes): {:02x?}", digest_bytes.len(), digest_bytes));
    
    // Generate ASCII art representation
    let ascii_art = hexagram_to_ascii(&hexagram);
    env::log(&format!("ASCII art representation ({} bytes):\n{}", ascii_art.len(), ascii_art));
    env::log(&format!("ASCII art bytes: {:02x?}", ascii_art.as_bytes()));
    
    // Assemble final output in correct order:
    // 1. Input digest (32 bytes)
    // 2. Marker byte (0xaa)
    // 3. Line values (6 bytes)
    // 4. ASCII art (47 bytes)
    let mut final_output = Vec::with_capacity(86); // 32 + 1 + 6 + 47 bytes
    
    // 1. Input digest (32 bytes)
    final_output.extend_from_slice(digest_bytes);
    env::log(&format!("Added input digest ({} bytes): {:02x?}", digest_bytes.len(), digest_bytes));
    env::log(&format!("Current output size: {}", final_output.len()));
    
    // 2. Marker byte (0xaa)
    final_output.push(DEV_MODE_MARKER);
    env::log(&format!("Added marker byte: 0x{:02x}", DEV_MODE_MARKER));
    env::log(&format!("Current output size: {}", final_output.len()));
    
    // 3. Line values (6 bytes)
    let line_values: Vec<u8> = hexagram.lines.iter().map(|&l| l as u8).collect();
    env::log(&format!("Line values to add (hex): {:02x?}", line_values));
    env::log(&format!("Line values to add (dec): {:?}", line_values));
    final_output.extend(&line_values);
    env::log(&format!("Added line values, current size: {}", final_output.len()));
    env::log(&format!("Line values in output (hex): {:02x?}", &final_output[33..39]));
    env::log(&format!("Line values in output (dec): {:?}", &final_output[33..39].iter().map(|&x| x).collect::<Vec<_>>()));
    
    // 4. ASCII art (47 bytes)
    env::log(&format!("ASCII art to add ({} bytes): {:02x?}", ascii_art.len(), ascii_art.as_bytes()));
    final_output.extend_from_slice(ascii_art.as_bytes());
    env::log(&format!("Added ASCII art, current size: {}", final_output.len()));
    env::log(&format!("ASCII art in output: {:02x?}", &final_output[39..]));
    
    // Log the final output structure
    env::log("\nFinal output structure:");
    env::log(&format!("1. Input digest (bytes 0-31): {:02x?}", &final_output[..32]));
    env::log(&format!("2. Marker byte (byte 32): 0x{:02x}", final_output[32]));
    env::log(&format!("3. Line values (bytes 33-38): {:02x?}", &final_output[33..39]));
    env::log(&format!("4. ASCII art (bytes 39-85): {:02x?}", &final_output[39..]));
    env::log(&format!("Total size: {} bytes", final_output.len()));
    
    // Verify output structure before committing
    if final_output.len() != 86 {
        env::log(&format!("❌ ERROR: Invalid output size! Expected 86 bytes, got {}", final_output.len()));
        env::log(&format!("- Input digest: {} bytes", digest_bytes.len()));
        env::log(&format!("- Marker byte: 1 byte"));
        env::log(&format!("- Line values: {} bytes", line_values.len()));
        env::log(&format!("- ASCII art: {} bytes", ascii_art.len()));
    }
    
    // Verify line values are valid
    let valid_lines = final_output[33..39].iter().all(|&x| (6..=9).contains(&x));
    if !valid_lines {
        env::log("❌ ERROR: Invalid line values detected!");
        env::log(&format!("Line values (hex): {:02x?}", &final_output[33..39]));
        env::log(&format!("Line values (dec): {:?}", &final_output[33..39].iter().map(|&x| x).collect::<Vec<_>>()));
        env::log("Each value must be between 6 and 9");
    }
    
    // Verify ASCII art length
    if ascii_art.len() != 47 {
        env::log(&format!("❌ ERROR: Invalid ASCII art length! Expected 47 bytes, got {}", ascii_art.len()));
        env::log(&format!("ASCII art: {:02x?}", ascii_art.as_bytes()));
    }
    
    // Commit the entire output at once
    env::log(&format!("Committing final output ({} bytes): {:02x?}", final_output.len(), final_output));
    env::commit_slice(&final_output);
    
    env::log(&format!("Hexagram generation complete. Total committed data: {} bytes", final_output.len()));
    env::log("Journal data structure:");
    env::log(&format!("- Input digest: {} bytes", digest_bytes.len()));
    env::log(&format!("- Structured output: {} bytes", 1 + hexagram.lines.len()));
    env::log(&format!("- ASCII art: {} bytes", ascii_art.len()));
}

fn generate_hexagram(random_seed: &[u8]) -> HexagramGeneration {
    let mut lines = [LineValue::default(); 6];
    
    env::log("Starting line generation...");
    for line_idx in 0..6 {
        // Generate each line using a different portion of the random seed
        let line_seed = &random_seed[line_idx*4..(line_idx+1)*4];
        env::log(&format!("Line {} seed ({} bytes): {:02x?}", line_idx + 1, line_seed.len(), line_seed));
        lines[line_idx] = generate_line_value(line_seed);
        env::log(&format!("Generated line {} = {:?} (value {})", 
            line_idx + 1, 
            lines[line_idx], 
            lines[line_idx] as u8
        ));
    }
    
    HexagramGeneration { lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagram_generation() {
        let random_seed = [42u8; 32];
        let hexagram = generate_hexagram(&random_seed);
        
        // Verify all lines have valid values
        for line in &hexagram.lines {
            assert!(matches!(line, 
                LineValue::YoungYang | 
                LineValue::OldYang | 
                LineValue::YoungYin | 
                LineValue::OldYin
            ));
        }
    }

    #[test]
    fn test_ascii_art_generation() {
        // Generate a hexagram with random lines
        let random_seed = [42u8; 32];
        let hexagram = generate_hexagram(&random_seed);
        let ascii_art = hexagram_to_ascii(&hexagram);
        
        // Split into lines
        let lines: Vec<&str> = ascii_art.split('\n').collect();
        
        // Validate structure
        assert_eq!(lines.len(), 6, "Should have exactly 6 lines");
        
        // Validate each line
        for (i, line) in lines.iter().enumerate() {
            // Each line should be exactly 7 chars
            assert_eq!(line.len(), 7, "Line {} should be 7 chars, got {}", i + 1, line.len());
            
            // Each line should only contain valid characters
            assert!(line.chars().all(|c| matches!(c, '-' | 'x' | 'o' | ' ')), 
                "Line {} contains invalid characters: {}", i + 1, line);
        }
        
        // Validate total size (6 lines * 7 chars + 5 newlines = 47 bytes)
        assert_eq!(ascii_art.len(), 47, "Total ASCII art should be 47 bytes");
    }
} 