//! Local add-on version comparison only. No update networking or installer execution.

fn numbers(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("")
        .split('.')
        .map(|p| p.trim().parse().unwrap_or(0))
        .collect()
}

/// `remote` is a higher version than `local` ("1.0.10" > "1.0.9").
pub fn is_newer(remote: &str, local: &str) -> bool {
    let (mut a, mut b) = (numbers(remote), numbers(local));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("v1.1", "1.0.9"));
        assert!(is_newer("1.0.10", "1.0.9"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("garbage", "1.0.0"));
    }

}
