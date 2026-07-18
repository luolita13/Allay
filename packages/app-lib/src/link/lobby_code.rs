//! Lobby code generation and parsing.
//!
//! Ported from PCL-CE's `LobbyCodeGenerator.cs`.
//! Format: `U/XXXX-XXXX-XXXX-XXXX` (21 chars, base34 encoding with mod-7 checksum).

use rand::Rng;

use super::types::LobbyInfo;

/// Base34 character set (I and O are excluded to avoid confusion with 1 and 0).
const CHARSET: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Prefix for all lobby codes.
const CODE_PREFIX: &str = "U/";

/// Prefix for EasyTier network names derived from lobby codes.
const NETWORK_NAME_PREFIX: &str = "scaffolding-mc-";

/// Generate a new random lobby code.
pub fn generate() -> LobbyInfo {
    let mut rng = rand::thread_rng();

    // Generate 13 random base34 digits (fits in ~66 bits).
    let mut digits: Vec<u8> = (0..13).map(|_| rng.gen_range(0..34u8)).collect();

    // Adjust the numeric value so that value % 7 == 0 (checksum).
    let remainder = value_mod(&digits, 7);
    if remainder != 0 {
        let adjustment = (7 - remainder) as u8;
        // Distribute the adjustment across the digits to keep them valid.
        let last = digits.last_mut().unwrap();
        *last = (*last + adjustment) % 34;
    }

    // Encode digits to characters.
    let encoded: String = digits.iter().map(|&d| CHARSET[d as usize] as char).collect();

    // Format: U/XXXX-XXXX-XXXX-XXXX
    let full_code = format!(
        "{prefix}{a}-{b}-{c}-{d}",
        prefix = CODE_PREFIX,
        a = &encoded[0..4],
        b = &encoded[4..8],
        c = &encoded[8..12],
        d = &encoded[12..13]
    );

    // Derive network name and secret from the encoded value.
    // Network name: "scaffolding-mc-" + first 9 chars
    // Network secret: last 4 chars
    let network_name = format!("{}{}", NETWORK_NAME_PREFIX, &encoded[0..9]);
    let network_secret = encoded[9..13].to_string();

    LobbyInfo {
        full_code,
        network_name,
        network_secret,
    }
}

/// Parse and validate a lobby code. Returns `LobbyInfo` on success.
pub fn parse(code: &str) -> Result<LobbyInfo, String> {
    let code = code.trim().to_uppercase();

    // Check prefix.
    if !code.starts_with(CODE_PREFIX) {
        return Err(format!(
            "Invalid lobby code: must start with '{}'",
            CODE_PREFIX
        ));
    }

    let body = &code[CODE_PREFIX.len()..];

    // Split into groups by '-'.
    let groups: Vec<&str> = body.split('-').collect();
    if groups.len() != 4 {
        return Err("Invalid lobby code: expected 4 groups separated by '-'".to_string());
    }

    // Validate group lengths: 4-4-4-1.
    let expected_lengths = [4, 4, 4, 1];
    for (i, (group, &expected)) in groups.iter().zip(expected_lengths.iter()).enumerate() {
        if group.len() != expected {
            return Err(format!(
                "Invalid lobby code: group {} has length {}, expected {}",
                i + 1,
                group.len(),
                expected
            ));
        }
    }

    // Concatenate all groups.
    let encoded: String = groups.concat();

    // Validate characters.
    let digits: Vec<u8> = encoded
        .chars()
        .map(|c| {
            CHARSET
                .iter()
                .position(|&d| d as char == c)
                .ok_or_else(|| format!("Invalid character in lobby code: '{}'", c))
                .map(|p| p as u8)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Verify checksum: value % 7 == 0.
    let remainder = value_mod(&digits, 7);
    if remainder != 0 {
        return Err("Invalid lobby code: checksum verification failed".to_string());
    }

    // Derive network name and secret.
    let network_name = format!("{}{}", NETWORK_NAME_PREFIX, &encoded[0..9]);
    let network_secret = encoded[9..13].to_string();

    Ok(LobbyInfo {
        full_code: code,
        network_name,
        network_secret,
    })
}

/// Compute `digits` interpreted as a base-34 number modulo `m`.
fn value_mod(digits: &[u8], m: u32) -> u32 {
    let mut result: u32 = 0;
    for &d in digits {
        result = (result * 34 + d as u32) % m;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_parse_roundtrip() {
        for _ in 0..100 {
            let info = generate();
            let parsed = parse(&info.full_code).unwrap();
            assert_eq!(parsed.network_name, info.network_name);
            assert_eq!(parsed.network_secret, info.network_secret);
        }
    }

    #[test]
    fn test_parse_invalid_prefix() {
        assert!(parse("V/AAAA-AAAA-AAAA-A").is_err());
    }

    #[test]
    fn test_parse_invalid_checksum() {
        // Change the last character to break the checksum.
        let info = generate();
        let mut code = info.full_code.chars().collect::<Vec<_>>();
        let last = code.last_mut().unwrap();
        *last = if *last == 'A' { 'B' } else { 'A' };
        let broken: String = code.into_iter().collect();
        assert!(parse(&broken).is_err());
    }
}
