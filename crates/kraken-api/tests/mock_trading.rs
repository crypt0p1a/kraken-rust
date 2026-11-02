//! Every trading call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, Signed, ok, orders_fixture, private_api};
use kraken_api::KrakenError;
use kraken_api::trading::{
    CloseTime, ClosedOrdersRequest, NewOrder, OpenOrdersRequest, OrderFlag, OrderSide, OrderStatus,
    OrderType, QueryOrdersRequest, TimeInForce, TradesHistoryRequest,
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
