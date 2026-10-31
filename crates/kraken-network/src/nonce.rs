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

    #[test]
    fn gives_unique_nonces_to_concurrent_threads() {
        let threads: Vec<_> = (0..8)
            .map(|_| std::thread::spawn(|| (0..500).map(|_| next_nonce()).collect::<Vec<_>>()))
            .collect();

        let mut all = Vec::new();
        for thread in threads {
            let nonces = thread.join().unwrap();
            assert!(nonces.windows(2).all(|pair| pair[0] < pair[1]));
            all.extend(nonces);
        }

        let count = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), count, "a nonce was given twice");
    }

    #[test]
    fn follows_the_clock() {
        let nonce = next_nonce();
        let now = now_millis();
        // ahead of the clock only by the nonces given faster than one per millisecond
        assert!(
            nonce >= now - 1_000 && nonce <= now + 100_000,
            "{nonce} vs {now}"
        );
    }
}
