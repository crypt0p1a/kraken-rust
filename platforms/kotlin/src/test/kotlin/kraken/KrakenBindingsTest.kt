package kraken

import java.math.BigDecimal
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.test.runTest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test
import uniffi.kraken_api.FfiConverterTypeDecimal
import uniffi.kraken_ffi.KrakenLogger
import uniffi.kraken_ffi.KrakenPrivateApi
import uniffi.kraken_ffi.KrakenPublicApi
import uniffi.kraken_ffi.KrakenRateLimiter
import uniffi.kraken_ffi.defaultRestOptions
import uniffi.kraken_network.KrakenException
import uniffi.kraken_network.RestOptions

/**
 * The Kotlin bindings of kraken-ffi, called against a local mocked Kraken: on the JVM
 * (platforms/kotlin) and on Android (platforms/android, instrumented tests)
 */
class KrakenBindingsTest {
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

    @Before
    fun start() = server.start()

    @After
    fun stop() = server.shutdown()

    private fun options(): RestOptions =
        defaultRestOptions().copy(host = server.url("/").toString().trimEnd('/'), timeoutMs = 5000uL)

    private fun limiter() = KrakenRateLimiter(10u, 1uL, null)

    private fun ok(body: String) = MockResponse().setResponseCode(200).setBody(body)

    @Test
    fun exposesTheDefaultOptions() {
        val options = defaultRestOptions()
        assertEquals("https://api.kraken.com", options.host)
        assertEquals(30_000uL, options.timeoutMs)
        assertTrue(options.rejectUnauthorized)
    }

    @Test
    fun theAmountsAreExactBigDecimals() = runTest {
        server.enqueue(ok(TICKER))
        val api = KrakenPublicApi(options(), limiter())

        val ticker = api.tickers(listOf("SHIBUSD")).getValue("SHIBUSD")

        assertEquals(BigDecimal(SMALL_PRICE), ticker.ask.price)
        assertEquals(BigDecimal(LARGE_VOLUME), ticker.volume.today)
        assertEquals(BigDecimal(LARGE_VOLUME), ticker.lastTrade.lotVolume)
        // every digit, which a Double doesn't keep
        assertEquals(LARGE_VOLUME, ticker.volume.today.toPlainString())
        assertNotEquals(LARGE_VOLUME, LARGE_VOLUME.toDouble().toString())
        assertEquals("/0/public/Ticker?pair=SHIBUSD", server.takeRequest().path)
    }

    @Test
    fun theDecimalsCrossTheFfiBothWays() {
        for (text in listOf(LARGE_VOLUME, SMALL_PRICE, "-0.5", "0", "1E-7", "123456789012345678.9012345678")) {
            val value = BigDecimal(text)
            val back = FfiConverterTypeDecimal.lift(FfiConverterTypeDecimal.lower(value))
            assertEquals(text, 0, value.compareTo(back))
        }
    }

    @Test
    fun thePrivateApiSignsAndDecodesTheBalances() = runTest {
        server.enqueue(
            ok("""{"error":[],"result":{"SHIB":{"balance":"$LARGE_VOLUME","hold_trade":"0.00000001"}}}"""),
        )
        val api = KrakenPrivateApi("key", "c2VjcmV0", options(), limiter())

        val balance = api.extendedBalance().getValue("SHIB")

        assertEquals(BigDecimal(LARGE_VOLUME), balance.balance)
        assertEquals(BigDecimal("0.00000001"), balance.holdTrade)
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

        try {
            api.tickers(listOf("NOPE"))
            fail("an error expected")
        } catch (error: KrakenException.Api) {
            assertEquals(200.toUShort(), error.status)
            assertEquals("EQuery:Unknown asset pair", error.body)
        }

        try {
            api.serverTime()
            fail("an error expected")
        } catch (error: KrakenException.Network) {
            assertEquals(503.toUShort(), error.status)
            assertEquals("unavailable", error.body)
        }
    }

    @Test
    fun anInvalidSecretIsRefused() {
        try {
            KrakenPrivateApi("key", "not base64!", options(), limiter())
            fail("an error expected")
        } catch (error: KrakenException.InvalidArgument) {
            assertTrue(error.reason, error.reason.contains("base64"))
        }
    }

    class Recorder : KrakenLogger {
        val logs = CopyOnWriteArrayList<Pair<String, String>>()

        override fun log(tag: String, message: String) {
            logs.add(tag to message)
        }
    }

    @Test
    fun theRateLimiterLogsToKotlin() = runTest {
        repeat(2) { server.enqueue(ok("""{"error":[],"result":{"unixtime":1,"rfc1123":"x"}}""")) }
        val recorder = Recorder()
        // a single token, refilled every 500ms: the second call waits for it, which is logged
        val api = KrakenPublicApi(options(), KrakenRateLimiter(1u, 500uL, recorder))

        repeat(2) { assertEquals(1L, api.serverTime().unixTime) }

        assertTrue(recorder.logs.all { it.first == "[RateLimitQueue]" })
        assertTrue(recorder.logs.toString(), recorder.logs.any { it.second.contains("No token available") })
    }
}
