use crate::ApiSecret;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};

impl ApiSecret {
    /// `API-Sign` of a private call: base64(HMAC-SHA512(path + SHA256(nonce + body), secret))
    ///
    /// - `path`: path of the URL, e.g. "/0/private/AddOrder"
    /// - `body`: the exact body sent, nonce included
    pub fn sign(&self, path: &str, nonce: u64, body: &str) -> String {
        let digest = Sha256::digest(format!("{nonce}{body}").as_bytes());

        let mut mac =
            Hmac::<Sha512>::new_from_slice(&self.bytes).expect("HMAC accepts any key length");
        mac.update(path.as_bytes());
        mac.update(&digest);
        BASE64.encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use crate::ApiSecret;

    #[test]
    fn signs_like_the_documentation() {
        // example of https://docs.kraken.com/api/docs/guides/spot-rest-auth
        let secret = ApiSecret::from_base64(
            "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==",
        )
        .unwrap();

        assert_eq!(
            secret.sign(
                "/0/private/AddOrder",
                1616492376594,
                "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25",
            ),
            "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="
        );
    }

    fn secret() -> ApiSecret {
        ApiSecret::from_base64("c2VjcmV0LWtleS1vZi10aGUtdGVzdHM=").unwrap()
    }

    #[test]
    fn signs_deterministically_in_base64_of_64_bytes() {
        let signature = secret().sign("/0/private/BalanceEx", 1, "nonce=1");
        assert_eq!(
            signature,
            secret().sign("/0/private/BalanceEx", 1, "nonce=1")
        );
        assert_eq!(signature.len(), 88);
        assert!(signature.ends_with("=="));
    }

    #[test]
    fn signs_the_path_the_nonce_and_the_body() {
        let reference = secret().sign("/0/private/BalanceEx", 1, "nonce=1");
        for other in [
            secret().sign("/0/private/Balance", 1, "nonce=1"),
            secret().sign("/0/private/BalanceEx", 2, "nonce=1"),
            secret().sign("/0/private/BalanceEx", 1, "nonce=1&a=b"),
            ApiSecret::from_base64("b3RoZXI=")
                .unwrap()
                .sign("/0/private/BalanceEx", 1, "nonce=1"),
        ] {
            assert_ne!(other, reference);
        }
    }

    #[test]
    fn signs_the_concatenation_of_the_nonce_and_the_body() {
        // SHA256(nonce + body), as documented by Kraken: the body starts with "nonce=", so two
        // different requests can't share a signature
        assert_eq!(
            secret().sign("/0/private/X", 12, "3"),
            secret().sign("/0/private/X", 1, "23")
        );
    }
}
