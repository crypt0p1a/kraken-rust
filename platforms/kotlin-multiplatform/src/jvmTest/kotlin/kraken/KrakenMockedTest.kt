package kraken

import com.ionspin.kotlin.bignum.decimal.BigDecimal
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import uniffi.kraken_ffi.KrakenPrivateApi
import uniffi.kraken_ffi.KrakenPublicApi
import uniffi.kraken_ffi.KrakenRateLimiter
import uniffi.kraken_ffi.defaultRestOptions
import uniffi.kraken_network.KrakenException
import uniffi.kraken_network.RestOptions

/** The Kotlin Multiplatform bindings on the JVM, against a local mocked Kraken */
class KrakenMockedTest {
    companion object {
        /** Beyond the ~16 significant digits of a Double */
        const val LARGE_VOLUME = "812345678901.12345678"
        const val SMALL_PRICE = "0.000012345678"

        val TICKER = """
            {"error":[],"result":{"SHIBUSD":{
             "a":["$SMALL_PRICE","1","1.000"],"b":["0.000012345600","1","1.000"],
             "c":["0.000012345678","$LARGE_VOLUME"],"v":["$LARGE_VOLUME","2177.75547658"],
             "p":["0.000012300000","0.000012200000"],"t":[37419,137908],
             "l":["0.000012000000","0.000011000000"],"h":["0.000013000000","0.000014000000"],
             "o":"0.000012100000"}}}
        """.trimIndent()
    }

    private val server = MockWebServer()

    @BeforeTest
    fun start() = server.start()

    @AfterTest
    fun stop() = server.shutdown()

    private fun options(): RestOptions =
        defaultRestOptions().copy(host = server.url("/").toString().trimEnd('/'), timeoutMs = 5000uL)

    private fun limiter() = KrakenRateLimiter(10u, 1uL, null)

    private fun ok(body: String) = MockResponse().setResponseCode(200).setBody(body)

    private fun decimal(text: String) = BigDecimal.parseString(text)

    @Test
    fun theAmountsAreExactBigDecimals() = runTest {
        server.enqueue(ok(TICKER))
        val api = KrakenPublicApi(options(), limiter())

        val ticker = api.tickers(listOf("SHIBUSD")).getValue("SHIBUSD")

        assertEquals(decimal(SMALL_PRICE), ticker.ask.price)
        assertEquals(decimal(LARGE_VOLUME), ticker.volume.today)
        assertEquals(decimal(LARGE_VOLUME), ticker.lastTrade.lotVolume)
        // every digit, which a Double doesn't keep
        assertEquals(LARGE_VOLUME, ticker.volume.today.toStringExpanded())
        assertTrue(LARGE_VOLUME.toDouble().toString() != LARGE_VOLUME)
        assertEquals("/0/public/Ticker?pair=SHIBUSD", server.takeRequest().path)
    }

    @Test
    fun thePrivateApiSignsAndDecodesTheBalances() = runTest {
        server.enqueue(
            ok("""{"error":[],"result":{"SHIB":{"balance":"$LARGE_VOLUME","hold_trade":"0.00000001"}}}"""),
        )
        val api = KrakenPrivateApi("key", "c2VjcmV0", options(), limiter())

        val balance = api.extendedBalance().getValue("SHIB")

        assertEquals(decimal(LARGE_VOLUME), balance.balance)
        assertEquals(decimal("0.00000001"), balance.holdTrade)
        assertNull(balance.credit)
        val request = server.takeRequest()
        assertEquals("POST", request.method)
        assertEquals("/0/private/BalanceEx", request.path)
        assertEquals("key", request.getHeader("API-Key"))
        assertTrue(request.getHeader("API-Sign").orEmpty().isNotEmpty())
        assertTrue(request.body.readUtf8().startsWith("nonce="))
    }

    @Test
    fun theErrorsOfKrakenAreThrown() = runTest {
        server.enqueue(ok("""{"error":["EQuery:Unknown asset pair"]}"""))
        server.enqueue(MockResponse().setResponseCode(503).setBody("unavailable"))
        val api = KrakenPublicApi(options(), limiter())

        val refused = assertFailsWith<KrakenException.Api> { api.tickers(listOf("NOPE")) }
        assertEquals(200.toUShort(), refused.status)
        assertEquals("EQuery:Unknown asset pair", refused.body)

        val unavailable = assertFailsWith<KrakenException.Network> { api.serverTime() }
        assertEquals(503.toUShort(), unavailable.status)
        assertEquals("unavailable", unavailable.body)
    }
}
