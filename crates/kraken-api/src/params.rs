//! Parameters shared by several endpoints

use kraken_network::params::comma_separated;
use serde::Serialize;

/// No pairs means all of them: an empty `pair=` is rejected by Kraken, a missing one isn't
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Pairs<'a> {
    #[serde(
        skip_serializing_if = "<[String]>::is_empty",
        serialize_with = "comma_separated"
    )]
    pub pair: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn omits_the_empty_pairs() {
        let pairs = ["XBTUSD".to_string(), "ETHUSD".to_string()];
        assert_eq!(
            encode(&Pairs { pair: &pairs }).unwrap(),
            "pair=XBTUSD%2CETHUSD"
        );
        assert_eq!(encode(&Pairs { pair: &[] }).unwrap(), "");
    }
}
