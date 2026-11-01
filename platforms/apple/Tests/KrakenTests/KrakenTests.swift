import Foundation
import Kraken
import XCTest

/// The Swift bindings of kraken-ffi, called against a local mocked Kraken
final class KrakenTests: XCTestCase {
    /// Beyond the ~16 significant digits of a Double
    static let largeVolume = "812345678901.12345678"
    static let smallPrice = "0.000012345678"

    static let ticker = """
    {"error":[],"result":{"SHIBUSD":{
     "a":["\(smallPrice)","1","1.000"],"b":["0.000012345600","1","1.000"],
     "c":["0.000012345678","\(largeVolume)"],"v":["\(largeVolume)","2177.75547658"],
     "p":["0.000012300000","0.000012200000"],"t":[37419,137908],
     "l":["0.000012000000","0.000011000000"],"h":["0.000013000000","0.000014000000"],
     "o":"0.000012100000"}}}
    """

    private var server: MockKraken!

    override func setUpWithError() throws {
        server = try MockKraken()
    }

    private func options() -> RestOptions {
        var options = defaultRestOptions()
        options.host = server.host
        options.timeoutMs = 5000
        return options
    }

    private func limiter() -> KrakenRateLimiter {
        KrakenRateLimiter(poolMaximum: 10, tokenIntervalMs: 1, logger: nil)
    }

    private func decimal(_ text: String) -> Decimal {
        Decimal(string: text, locale: Locale(identifier: "en_US_POSIX"))!
    }

    func testExposesTheDefaultOptions() {
        let options = defaultRestOptions()
        XCTAssertEqual(options.host, "https://api.kraken.com")
        XCTAssertEqual(options.timeoutMs, 30000)
        XCTAssertTrue(options.rejectUnauthorized)
    }

    func testTheAmountsAreExactDecimals() async throws {
        server.answer("/0/public/Ticker", body: Self.ticker)
        let api = try KrakenPublicApi(options: options(), rateLimiter: limiter())

        let ticker = try await api.tickers(pairs: ["SHIBUSD"])["SHIBUSD"]!

        XCTAssertEqual(ticker.ask.price, decimal(Self.smallPrice))
        XCTAssertEqual(ticker.volume.today, decimal(Self.largeVolume))
        XCTAssertEqual(ticker.lastTrade.lotVolume, decimal(Self.largeVolume))
        // every digit, which a Double doesn't keep
        XCTAssertEqual(
            NSDecimalNumber(decimal: ticker.volume.today)
                .description(withLocale: Locale(identifier: "en_US_POSIX")),
            Self.largeVolume
        )
        XCTAssertNotEqual(String(Double(Self.largeVolume)!), Self.largeVolume)
        XCTAssertEqual(server.requests, ["GET /0/public/Ticker?pair=SHIBUSD HTTP/1.1"])
    }

    func testTheDecimalsCrossTheFfiBothWays() throws {
        for text in [Self.largeVolume, Self.smallPrice, "-0.5", "0", "0.00000015", "123456789012345678.9012345678"] {
            let value = decimal(text)
            XCTAssertEqual(try FfiConverterTypeDecimal.lift(FfiConverterTypeDecimal.lower(value)), value, text)
        }
    }

    func testThePrivateApiSignsAndDecodesTheBalances() async throws {
        server.answer("/0/private/BalanceEx", body: """
        {"error":[],"result":{"SHIB":{"balance":"\(Self.largeVolume)","hold_trade":"0.00000001"}}}
        """)
        let api = try KrakenPrivateApi(
            apiKey: "key", apiSecret: "c2VjcmV0", options: options(), rateLimiter: limiter()
        )

        let balances = try await api.extendedBalance()

        XCTAssertEqual(balances["SHIB"]?.balance, decimal(Self.largeVolume))
        XCTAssertEqual(balances["SHIB"]?.holdTrade, decimal("0.00000001"))
        XCTAssertNil(balances["SHIB"]?.credit)
        XCTAssertEqual(server.requests, ["POST /0/private/BalanceEx HTTP/1.1"])
        let request = server.received[0].lowercased()
        XCTAssertTrue(request.contains("\r\napi-key: key\r\n"), request)
        XCTAssertTrue(request.contains("\r\napi-sign: "), request)
        XCTAssertTrue(request.contains("\r\n\r\nnonce="), request)
    }

    func testTheErrorsOfKrakenAreThrown() async throws {
        server.answer("/0/public/Ticker", body: #"{"error":["EQuery:Unknown asset pair"]}"#)
        server.answer("/0/public/Time", status: 503, body: "unavailable")
        let api = try KrakenPublicApi(options: options(), rateLimiter: limiter())

        do {
            _ = try await api.tickers(pairs: ["NOPE"])
            XCTFail("an error expected")
        } catch let KrakenError.Api(status, body) {
            XCTAssertEqual(status, 200)
            XCTAssertEqual(body, "EQuery:Unknown asset pair")
        }

        do {
            _ = try await api.serverTime()
            XCTFail("an error expected")
        } catch let KrakenError.Network(status, body) {
            XCTAssertEqual(status, 503)
            XCTAssertEqual(body, "unavailable")
        }
    }

    func testAnInvalidSecretIsRefused() {
        XCTAssertThrowsError(
            try KrakenPrivateApi(apiKey: "key", apiSecret: "not base64!", options: options(), rateLimiter: limiter())
        ) { error in
            guard case KrakenError.InvalidArgument = error else {
                return XCTFail("\(error)")
            }
        }
    }

    final class Recorder: KrakenLogger, @unchecked Sendable {
        private let lock = NSLock()
        private var logs: [(String, String)] = []

        func log(tag: String, message: String) {
            lock.lock()
            logs.append((tag, message))
            lock.unlock()
        }

        var messages: [(String, String)] {
            lock.lock()
            defer { lock.unlock() }
            return logs
        }
    }

    func testTheRateLimiterLogsToSwift() async throws {
        server.answer("/0/public/Time", body: #"{"error":[],"result":{"unixtime":1,"rfc1123":"x"}}"#)
        let recorder = Recorder()
        // a single token, refilled every 500ms: the second call waits for it, which is logged
        let limiter = KrakenRateLimiter(poolMaximum: 1, tokenIntervalMs: 500, logger: recorder)
        let api = try KrakenPublicApi(options: options(), rateLimiter: limiter)

        for _ in 0..<2 {
            let time = try await api.serverTime()
            XCTAssertEqual(time.unixTime, 1)
        }

        XCTAssertTrue(recorder.messages.allSatisfy { $0.0 == "[RateLimitQueue]" })
        XCTAssertTrue(recorder.messages.contains { $0.1.contains("No token available") }, "\(recorder.messages)")
    }
}
