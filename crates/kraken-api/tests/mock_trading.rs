//! Every trading call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, JsonIs, Signed, SignedJson, ok, orders_fixture, private_api};
use kraken_api::KrakenError;
use kraken_api::trading::{
    AmendOrder, AmendType, CloseTime, ClosedOrdersRequest, EditOrder, NewOrder, OpenOrdersRequest,
    OpenPositionsRequest, OrderAmendsRequest, OrderFlag, OrderReference, OrderSide, OrderStatus,
    OrderType, QueryOrdersRequest, QueryTradesRequest, TimeInForce, TradesHistoryRequest,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

async fn mock_open_orders(server: &MockServer, form: Vec<(&'static str, &'static str)>) {
    Mock::given(method("POST"))
        .and(path("/0/private/OpenOrders"))
        .and(Signed)
        .and(FormIs(form))
        .respond_with(ok(json!({ "open": orders_fixture() })))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn open_orders() {
    let server = MockServer::start().await;
    mock_open_orders(
        &server,
        vec![("trades", "true"), ("cl_ord_id", "mgr1_buy_1")],
    )
    .await;

    let orders = private_api(&server)
        .open_orders(&OpenOrdersRequest {
            trades: Some(true),
            cl_ord_id: Some("mgr1_buy_1".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(orders.len(), 2);
    // most recent first, with their txid
    assert_eq!(orders[0].txid, "OQCLML-BW3P3-BUCMWZ");
    assert_eq!(orders[1].txid, "OB5VMB-B4U2U-DK2WRW");
}

#[tokio::test]
async fn open_orders_for_pair() {
    let server = MockServer::start().await;
    mock_open_orders(&server, vec![]).await;

    let orders = private_api(&server)
        .open_orders_for_pair("ETHUSD")
        .await
        .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].description.side, OrderSide::Sell);
}

#[tokio::test]
async fn closed_orders() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/ClosedOrders"))
        .and(Signed)
        .and(FormIs(vec![
            ("start", "1688000000"),
            ("end", "1688700000"),
            ("ofs", "50"),
            ("closetime", "close"),
        ]))
        .respond_with(ok(json!({ "closed": orders_fixture(), "count": 52 })))
        .expect(1)
        .mount(&server)
        .await;

    let closed = private_api(&server)
        .closed_orders(&ClosedOrdersRequest {
            start: Some(1688000000),
            end: Some(1688700000),
            offset: Some(50),
            close_time: Some(CloseTime::Close),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(closed.count, 52);
    assert_eq!(closed.orders.len(), 2);
}

#[tokio::test]
async fn query_orders() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/QueryOrders"))
        .and(Signed)
        .and(FormIs(vec![
            ("txid", "OQCLML-BW3P3-BUCMWZ,OB5VMB-B4U2U-DK2WRW"),
            ("trades", "true"),
        ]))
        .respond_with(ok(orders_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let orders = private_api(&server)
        .query_orders(&QueryOrdersRequest {
            txids: vec!["OQCLML-BW3P3-BUCMWZ".into(), "OB5VMB-B4U2U-DK2WRW".into()],
            trades: Some(true),
            userref: None,
        })
        .await
        .unwrap();
    assert_eq!(orders.len(), 2);
}

#[tokio::test]
async fn order() {
    let server = MockServer::start().await;
    Mock::given(path("/0/private/QueryOrders"))
        .and(Signed)
        .and(FormIs(vec![("txid", "OB5VMB-B4U2U-DK2WRW")]))
        .respond_with(ok(orders_fixture()))
        .mount(&server)
        .await;
    Mock::given(path("/0/private/QueryOrders"))
        .and(FormIs(vec![("txid", "UNKNOWN")]))
        .respond_with(ok(json!({})))
        .mount(&server)
        .await;

    let api = private_api(&server);
    let order = api.order("OB5VMB-B4U2U-DK2WRW").await.unwrap().unwrap();
    assert_eq!(order.status, OrderStatus::Closed);
    assert_eq!(api.order("UNKNOWN").await.unwrap(), None);
}

fn limit_buy() -> NewOrder {
    NewOrder {
        pair: "XBTUSD".into(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        volume: "1.25".into(),
        price: Some("27500.0".into()),
        price2: None,
        oflags: Some(vec![OrderFlag::Post]),
        time_in_force: Some(TimeInForce::Gtd),
        start_time: None,
        expire_time: Some("+43200".into()),
        cl_ord_id: Some("mgr1_buy_1".into()),
        userref: None,
        validate: None,
    }
}

#[tokio::test]
async fn new_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/AddOrder"))
        .and(Signed)
        .and(FormIs(vec![
            ("pair", "XBTUSD"),
            ("type", "buy"),
            ("ordertype", "limit"),
            ("volume", "1.25"),
            ("price", "27500.0"),
            ("oflags", "post"),
            ("timeinforce", "GTD"),
            ("expiretm", "+43200"),
            ("cl_ord_id", "mgr1_buy_1"),
        ]))
        .respond_with(ok(json!({
            "descr": {"order": "buy 1.25000000 XBTUSD @ limit 27500.0"},
            "txid": ["OU22CG-KLAF2-FWUDD7"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let answer = private_api(&server).new_order(&limit_buy()).await.unwrap();
    assert_eq!(answer.txids, vec!["OU22CG-KLAF2-FWUDD7"]);
}

#[tokio::test]
async fn new_order_rejects_the_invalid_requests_without_calling() {
    let server = MockServer::start().await;
    Mock::given(path("/0/private/AddOrder"))
        .respond_with(ok(json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let api = private_api(&server);

    let without_expire_time = NewOrder {
        expire_time: None,
        ..limit_buy()
    };
    assert!(matches!(
        api.new_order(&without_expire_time).await,
        Err(KrakenError::InvalidArgument { .. })
    ));

    let with_both_ids = NewOrder {
        userref: Some(1),
        ..limit_buy()
    };
    assert!(matches!(
        api.new_order(&with_both_ids).await,
        Err(KrakenError::InvalidArgument { .. })
    ));
}

#[tokio::test]
async fn trades_history() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/TradesHistory"))
        .and(Signed)
        .and(FormIs(vec![
            ("pair", "XBTUSD"),
            ("limit", "2"),
            ("with_cursor", "true"),
        ]))
        .respond_with(ok(json!({"count": 0, "cursor": {"next": "page-2"}, "trades": {
            "THVRQM-33VKH-UCI7BS": {"ordertxid": "OQCLML-BW3P3-BUCMWZ", "pair": "XXBTZUSD",
                "time": 1688667796.8802, "type": "buy", "ordertype": "limit", "price": "30010.0",
                "cost": "616.79361", "fee": "1.60366", "vol": "0.02055308", "misc": "",
                "trade_id": 93748276, "maker": true}
        }})))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .trades_history(&TradesHistoryRequest {
            pair: Some("XBTUSD".into()),
            limit: Some(2),
            with_cursor: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.trades[0].txid, "THVRQM-33VKH-UCI7BS");
    assert_eq!(page.cursor.unwrap().next.as_deref(), Some("page-2"));
}

#[tokio::test]
async fn query_trades() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/QueryTrades"))
        .and(Signed)
        .and(FormIs(vec![
            ("txid", "THVRQM-33VKH-UCI7BS"),
            ("trades", "true"),
        ]))
        .respond_with(ok(json!({
            "THVRQM-33VKH-UCI7BS": {"ordertxid": "OQCLML-BW3P3-BUCMWZ", "pair": "XXBTZUSD",
                "time": 1688667796.8802, "type": "sell", "ordertype": "limit", "price": "30010.0",
                "cost": "616.79361", "fee": "1.60366", "vol": "0.02055308", "misc": "",
                "trade_id": 93748276}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let trades = private_api(&server)
        .query_trades(&QueryTradesRequest {
            txids: vec!["THVRQM-33VKH-UCI7BS".into()],
            trades: Some(true),
            ledgers: None,
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(trades[0].side, OrderSide::Sell);
}

#[tokio::test]
async fn order_amends() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/OrderAmends"))
        .and(Signed)
        .and(FormIs(vec![("order_id", "OQCLML-BW3P3-BUCMWZ")]))
        .respond_with(ok(
            json!({"count": 1, "amends": [{"amend_id": "TM2EBP-HGEUP-36CIRK",
            "amend_type": "original", "order_qty": "1.25", "remaining_qty": "1.25",
            "limit_price": "30010.0", "post_only": false, "timestamp": 1688666559897u64}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let amends = private_api(&server)
        .order_amends(&OrderAmendsRequest {
            order_id: "OQCLML-BW3P3-BUCMWZ".into(),
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(amends.amends[0].amend_type, AmendType::Original);
}

#[tokio::test]
async fn open_positions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/OpenPositions"))
        .and(Signed)
        .and(FormIs(vec![("docalcs", "true")]))
        .respond_with(ok(
            json!({"TF5GVO-T7ZZ2-6NBKBI": {"ordertxid": "OLWNFG-LLH4R-D6SFFP",
            "posstatus": "open", "pair": "XXBTZUSD", "time": 1605280097.8294, "type": "sell",
            "ordertype": "market", "cost": "100", "fee": "0.26", "vol": "0.01",
            "vol_closed": "0", "margin": "20", "value": "99", "net": "+0.74", "misc": "",
            "oflags": ""}}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let positions = private_api(&server)
        .open_positions(&OpenPositionsRequest {
            docalcs: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(positions[0].txid, "TF5GVO-T7ZZ2-6NBKBI");
    assert_eq!(positions[0].side, OrderSide::Sell);
}

#[tokio::test]
async fn open_positions_by_market() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/OpenPositions"))
        .and(Signed)
        .and(FormIs(vec![("consolidation", "market")]))
        .respond_with(ok(
            json!([{"pair": "XXBTZUSD", "class": "margin", "positions": "2",
            "type": "buy", "leverage": "n/a", "cost": "100", "fee": "0.26", "vol": "0.01",
            "vol_closed": "0", "margin": "20"}]),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let positions = private_api(&server)
        .open_positions_by_market(None)
        .await
        .unwrap();
    assert_eq!(positions[0].positions, 2);
    assert_eq!(positions[0].leverage, "n/a");
}

#[tokio::test]
async fn websockets_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/GetWebSocketsToken"))
        .and(Signed)
        .and(FormIs(vec![]))
        .respond_with(ok(
            json!({"token": "1Dwc4lzSwNWOAwkMdqhssNNFhs1ed606d1WcF3XfEMw",
            "expires": 900}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let token = private_api(&server).websockets_token().await.unwrap();
    assert_eq!(token.expires, 900);
}

#[tokio::test]
async fn cancel_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelOrder"))
        .and(Signed)
        .and(FormIs(vec![("cl_ord_id", "mgr1_buy_1")]))
        .respond_with(ok(json!({"count": 1})))
        .expect(1)
        .mount(&server)
        .await;

    let cancelled = private_api(&server)
        .cancel_order(&OrderReference::ClOrdId {
            cl_ord_id: "mgr1_buy_1".into(),
        })
        .await
        .unwrap();
    assert_eq!(cancelled.count, 1);
}

#[tokio::test]
async fn cancel_all() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelAll"))
        .and(Signed)
        .and(FormIs(vec![]))
        .respond_with(ok(json!({"count": 4, "pending": false})))
        .expect(1)
        .mount(&server)
        .await;

    let cancelled = private_api(&server).cancel_all().await.unwrap();
    assert_eq!(cancelled.count, 4);
    assert_eq!(cancelled.pending, Some(false));
}

#[tokio::test]
async fn cancel_all_orders_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelAllOrdersAfter"))
        .and(Signed)
        .and(FormIs(vec![("timeout", "60")]))
        .respond_with(ok(json!({"currentTime": "2023-03-24T17:41:56Z",
            "triggerTime": "2023-03-24T17:42:56Z"})))
        .expect(1)
        .mount(&server)
        .await;

    let timer = private_api(&server)
        .cancel_all_orders_after(60)
        .await
        .unwrap();
    assert_eq!(timer.current_time, "2023-03-24T17:41:56Z");
}

#[tokio::test]
async fn cancel_order_batch_is_signed_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelOrderBatch"))
        .and(SignedJson)
        .and(JsonIs(json!({"orders": ["OG5V2Y-RYKVL-DT3V3B", 45],
            "cl_ord_ids": ["mgr1_buy_1"]})))
        .respond_with(ok(json!({"count": 3})))
        .expect(1)
        .mount(&server)
        .await;

    let cancelled = private_api(&server)
        .cancel_order_batch(&[
            OrderReference::Txid {
                txid: "OG5V2Y-RYKVL-DT3V3B".into(),
            },
            OrderReference::Userref { userref: 45 },
            OrderReference::ClOrdId {
                cl_ord_id: "mgr1_buy_1".into(),
            },
        ])
        .await
        .unwrap();
    assert_eq!(cancelled.count, 3);
}

#[tokio::test]
async fn amend_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/AmendOrder"))
        .and(Signed)
        .and(FormIs(vec![
            ("cl_ord_id", "mgr1_buy_1"),
            ("limit_price", "30000.0"),
        ]))
        .respond_with(ok(json!({"amend_id": "TJSMEH-AA67V-YUSQ6O"})))
        .expect(1)
        .mount(&server)
        .await;

    let api = private_api(&server);
    let amend_id = api
        .amend_order(&AmendOrder {
            cl_ord_id: Some("mgr1_buy_1".into()),
            limit_price: Some("30000.0".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(amend_id, "TJSMEH-AA67V-YUSQ6O");

    // neither or both identifiers: refused before any request
    for request in [
        AmendOrder::default(),
        AmendOrder {
            txid: Some("O".into()),
            cl_ord_id: Some("C".into()),
            ..Default::default()
        },
    ] {
        assert!(matches!(
            api.amend_order(&request).await,
            Err(KrakenError::InvalidArgument { .. })
        ));
    }
}

#[tokio::test]
async fn edit_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/EditOrder"))
        .and(Signed)
        .and(FormIs(vec![
            ("txid", "OHYO67-6LP66-HMQ437"),
            ("pair", "XBTGBP"),
            ("volume", "0.0003"),
        ]))
        .respond_with(ok(json!({"status": "ok", "txid": "OFVXHJ-KPQ3B-VS7ELA",
            "originaltxid": "OHYO67-6LP66-HMQ437", "volume": "0.00030000",
            "orders_cancelled": 1, "descr": {"order": "buy 0.00030000 XXBTZGBP @ limit 19500.0"}})))
        .expect(1)
        .mount(&server)
        .await;

    let edited = private_api(&server)
        .edit_order(&EditOrder {
            txid: "OHYO67-6LP66-HMQ437".into(),
            pair: "XBTGBP".into(),
            volume: Some("0.0003".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(edited.txid.as_deref(), Some("OFVXHJ-KPQ3B-VS7ELA"));
}
