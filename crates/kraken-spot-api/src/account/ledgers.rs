use super::RebaseMultiplier;
use crate::{Cursor, KrakenError, lenient};
use kraken_network::PrivateClient;
use kraken_network::params::comma_separated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of a ledger entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum LedgerType {
    None,
    Trade,
    Deposit,
    Withdrawal,
    Transfer,
    Margin,
    Adjustment,
    Rollover,
    Spend,
    Receive,
    Settled,
    Credit,
    Staking,
    Reward,
    Dividend,
    Sale,
    Conversion,
    NftTrade,
    NftCreatorFee,
    NftRebate,
    CustodyTransfer,
    #[serde(other)]
    Unknown,
}

/// An entry of the ledger: a change of the balance of an asset
///
/// <https://docs.kraken.com/api/docs/rest-api/get-ledgers>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct LedgerEntry {
    /// Kraken identifier of the entry: the key of the answers, copied here
    #[serde(skip)]
    pub id: String,
    /// The transaction (trade, deposit, ...) of the entry
    pub refid: String,
    /// UTC timestamp in seconds, with decimals
    #[serde(deserialize_with = "lenient::number")]
    pub time: f64,
    #[serde(rename = "type")]
    pub entry_type: LedgerType,
    /// More about the type, e.g. "spottostaking"
    #[serde(default)]
    pub subtype: String,
    /// Asset class, e.g. "currency"
    pub aclass: String,
    /// e.g. "ZUSD"
    pub asset: String,
    /// Signed change of the balance
    #[serde(deserialize_with = "lenient::decimal")]
    pub amount: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub fee: Decimal,
    /// Balance after the entry
    #[serde(deserialize_with = "lenient::decimal")]
    pub balance: Decimal,
    /// The amount in raw units of the token: only for the tokenized assets
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub amount_token: Option<Decimal>,
}

/// Entries keyed by id → entries holding their id, most recent first
fn with_ids(entries: HashMap<String, LedgerEntry>) -> Vec<LedgerEntry> {
    crate::keyed::most_recent_first(
        entries,
        |id, entry| LedgerEntry { id, ..entry },
        |it| it.time,
    )
}

/// The types of entries the ledgers can be filtered by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum LedgerFilter {
    All,
    Trade,
    Deposit,
    Withdrawal,
    Transfer,
    Margin,
    Adjustment,
    Rollover,
    Credit,
    Settled,
    Staking,
    Dividend,
    Sale,
    NftRebate,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ledgers>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct LedgersRequest {
    /// e.g. ["XXBT", "ZUSD"]: all of them when empty
    #[serde(
        rename = "asset",
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = []))]
    pub assets: Vec<String>,
    /// Asset class of `assets`, e.g. "currency"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub aclass: Option<String>,
    /// All when missing
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub entry_type: Option<LedgerFilter>,
    /// UTC timestamp in seconds or ledger id (exclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start: Option<i64>,
    /// UTC timestamp in seconds or ledger id (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end: Option<i64>,
    /// Offset of the page of 50 entries: deprecated by Kraken, prefer `with_cursor`
    #[serde(rename = "ofs", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub offset: Option<i32>,
    /// Pagination by cursor instead of `offset`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub with_cursor: Option<bool>,
    /// The `cursor.next` of the previous page: requires `with_cursor`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cursor: Option<String>,
    /// No count of the entries: faster for the accounts with many entries
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub without_count: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// A page of 50 ledger entries at most
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Ledgers {
    /// Most recent first
    pub entries: Vec<LedgerEntry>,
    /// Entries matching the request: None with `without_count` or `with_cursor`
    pub count: Option<i64>,
    /// Only with `with_cursor`, when more entries are available
    pub cursor: Option<Cursor>,
}

#[derive(Deserialize)]
struct LedgersAnswer {
    #[serde(default)]
    ledger: HashMap<String, LedgerEntry>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    count: Option<i64>,
    #[serde(default)]
    cursor: Option<Cursor>,
}

/// Requires the "Query ledger entries" permission
pub(crate) async fn ledgers(
    client: &PrivateClient,
    request: &LedgersRequest,
) -> Result<Ledgers, KrakenError> {
    let answer: LedgersAnswer = client.call_with("Ledgers", Some(request)).await?;
    Ok(Ledgers {
        entries: with_ids(answer.ledger),
        count: answer.count,
        cursor: answer.cursor,
    })
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ledgers-info>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct QueryLedgersRequest {
    /// Up to 20 identifiers of ledger entries
    #[serde(rename = "id", serialize_with = "comma_separated")]
    pub ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// The entries, most recent first. Requires the "Query ledger entries" permission
pub(crate) async fn query_ledgers(
    client: &PrivateClient,
    request: &QueryLedgersRequest,
) -> Result<Vec<LedgerEntry>, KrakenError> {
    let entries: HashMap<String, LedgerEntry> =
        client.call_with("QueryLedgers", Some(request)).await?;
    Ok(with_ids(entries))
}

#[cfg(test)]
pub(crate) const SAMPLE: &str = r#"{
    "L4UESK-KG3EQ-UFO4T5":{"refid":"TJKLXX-PGMUI-4NTLXU","time":1688464484.1787,"type":"trade",
        "subtype":"","aclass":"currency","asset":"ZGBP","amount":"-24.5000","fee":"0.0490",
        "balance":"459567.9171"},
    "LMKZCZ-Z3GVL-CXKK4H":{"refid":"TBZIP2-F6QOU-TMB6FY","time":1688444262.8888,"type":"trade",
        "subtype":"","aclass":"currency","asset":"XXBT","amount":"0.0000000001","fee":"0.0000000000",
        "balance":"0.0000000001"},
    "LFOVVR-6FX3E-4HCVOR":{"refid":"FTzhVSJ-qQ2NJR2vmIhMzZm3DjE5Gr","time":1688444262.0,
        "type":"earn","subtype":"autoallocation","aclass":"currency","asset":"ETH",
        "amount":"0.5","fee":"0","balance":"0.5"}
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_every_parameter() {
        let request = LedgersRequest {
            assets: vec!["XXBT".into(), "ZUSD".into()],
            aclass: Some("currency".into()),
            entry_type: Some(LedgerFilter::NftRebate),
            start: Some(1),
            end: Some(2),
            offset: Some(0),
            with_cursor: Some(true),
            cursor: Some("abc".into()),
            without_count: Some(true),
            rebase_multiplier: Some(RebaseMultiplier::Rebased),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "asset=XXBT%2CZUSD&aclass=currency&type=nft_rebate&start=1&end=2&ofs=0\
             &with_cursor=true&cursor=abc&without_count=true&rebase_multiplier=rebased"
        );
        assert_eq!(encode(&LedgersRequest::default()).unwrap(), "");
    }

    #[test]
    fn encodes_the_query() {
        let request = QueryLedgersRequest {
            ids: vec!["L4UESK-KG3EQ-UFO4T5".into(), "LMKZCZ-Z3GVL-CXKK4H".into()],
            rebase_multiplier: None,
        };
        assert_eq!(
            encode(&request).unwrap(),
            "id=L4UESK-KG3EQ-UFO4T5%2CLMKZCZ-Z3GVL-CXKK4H"
        );
    }

    #[test]
    fn decodes_the_entries_most_recent_first() {
        let answer: LedgersAnswer =
            serde_json::from_str(&format!(r#"{{"ledger":{SAMPLE},"count":3}}"#)).unwrap();
        let entries = with_ids(answer.ledger);
        assert_eq!(entries[0].id, "L4UESK-KG3EQ-UFO4T5");
        assert_eq!(entries[0].amount, dec!(-24.5));
        assert_eq!(entries[0].entry_type, LedgerType::Trade);
        assert_eq!(entries[1].balance.to_string(), "0.0000000001");
        // a type unknown to this version
        assert_eq!(entries[2].entry_type, LedgerType::Unknown);
        assert_eq!(answer.count, Some(3));
    }

    #[test]
    fn decodes_every_type() {
        for (text, entry_type) in [
            ("none", LedgerType::None),
            ("rollover", LedgerType::Rollover),
            ("nfttrade", LedgerType::NftTrade),
            ("nftcreatorfee", LedgerType::NftCreatorFee),
            ("nftrebate", LedgerType::NftRebate),
            ("custodytransfer", LedgerType::CustodyTransfer),
            ("conversion", LedgerType::Conversion),
        ] {
            assert_eq!(
                serde_json::from_value::<LedgerType>(text.into()).unwrap(),
                entry_type
            );
        }
    }

    #[test]
    fn decodes_a_page_without_count() {
        let answer: LedgersAnswer =
            serde_json::from_str(r#"{"ledger":{},"count":null,"cursor":{"next":"n"}}"#).unwrap();
        assert_eq!(answer.count, None);
        assert_eq!(answer.cursor.unwrap().next.as_deref(), Some("n"));
    }
}
