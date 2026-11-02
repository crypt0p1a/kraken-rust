package kraken

import com.ionspin.kotlin.bignum.decimal.BigDecimal
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import uniffi.kraken_ffi.KrakenLogger
import uniffi.kraken_ffi.KrakenPrivateApi
import uniffi.kraken_ffi.KrakenPublicApi
import uniffi.kraken_ffi.KrakenRateLimiter
import uniffi.kraken_ffi.defaultRestOptions
import uniffi.kraken_network.KrakenException
import uniffi.kraken_network.RestOptions

/** A Decimal lowered to Rust, then lifted back: the converter of the target */
expect fun decimalRoundTrip(value: BigDecimal): BigDecimal

/**
 * The Kotlin Multiplatform bindings of kraken-ffi, on every target (JVM, Kotlin/Native): what
 * doesn't need a server. The calls to a mocked Kraken are in jvmTest
 */
class KrakenMultiplatformTest {
    /** Nothing listens on the port 1 of the loopback: the connection is refused */
    private fun unreachable(): RestOptions = defaultRestOptions().copy(host = "http://127.0.0.1:1", timeoutMs = 5000uL)

    @Test
    fun exposesTheDefaultOptions() {
        val options = defaultRestOptions()
        assertEquals("https://api.kraken.com", options.host)
        assertEquals(30_000uL, options.timeoutMs)
        assertTrue(options.rejectUnauthorized)
    }

    @Test
    fun theDecimalsCrossTheFfiBothWays() {
        // beyond the ~16 significant digits of a Double
        for (text in listOf("812345678901.12345678", "0.000012345678", "-0.5", "0", "0.0000001", "123456789012345678.9012345678")) {
            val value = BigDecimal.parseString(text)
            val back = decimalRoundTrip(value)
            assertEquals(0, value.compareTo(back), text)
            assertEquals(value.toStringExpanded(), back.toStringExpanded(), text)
        }
    }

    @Test
    fun anInvalidSecretIsRefused() {
        val error = assertFailsWith<KrakenException.InvalidArgument> {
            KrakenPrivateApi("key", "not base64!", defaultRestOptions(), KrakenRateLimiter(10u, 1uL, null))
        }
        assertTrue(error.reason.contains("base64"), error.reason)
    }

    @Test
    fun aServerOutOfReachIsATransportError() = runTest {
        val api = KrakenPublicApi(unreachable(), KrakenRateLimiter(10u, 1uL, null))
        assertFailsWith<KrakenException.Transport> { api.serverTime() }
    }

    class Recorder : KrakenLogger {
        val logs = mutableListOf<Pair<String, String>>()

        override fun log(tag: String, message: String) {
            logs.add(tag to message)
        }
    }

    @Test
    fun theRateLimiterLogsToKotlin() = runTest {
        val recorder = Recorder()
        // a single token, refilled every 500ms: the second call waits for it, which is logged
        val api = KrakenPublicApi(unreachable(), KrakenRateLimiter(1u, 500uL, recorder))

        repeat(2) { assertFailsWith<KrakenException.Transport> { api.serverTime() } }

        assertTrue(recorder.logs.isNotEmpty())
        assertTrue(recorder.logs.all { it.first == "[RateLimitQueue]" }, recorder.logs.toString())
        assertTrue(recorder.logs.any { it.second.contains("No token available") }, recorder.logs.toString())
    }
}
