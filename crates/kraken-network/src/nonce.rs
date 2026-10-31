use std::sync::atomic::{AtomicU64, Ordering};

/// Last nonce given, shared by every client of the process
static LAST_NONCE: AtomicU64 = AtomicU64::new(0);

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|it| it.as_millis() as u64)
        .unwrap_or_default()
}

/// Strictly increasing nonce, based on the current time in milliseconds. Kraken rejects a nonce
/// lower or equal to the previous one of the same API key ("EAPI:Invalid nonce")
pub(crate) fn next_nonce() -> u64 {
    let now = now_millis();
    let previous = LAST_NONCE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |last| {
            Some(now.max(last + 1))
        })
        .unwrap_or_default();

    now.max(previous + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gives_increasing_nonces() {
        let nonces: Vec<u64> = (0..100).map(|_| next_nonce()).collect();
        assert!(nonces.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
