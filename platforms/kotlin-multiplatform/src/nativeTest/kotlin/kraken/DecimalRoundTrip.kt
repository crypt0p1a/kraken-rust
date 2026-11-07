package kraken

import com.ionspin.kotlin.bignum.decimal.BigDecimal
import uniffi.kraken_spot_api.FfiConverterTypeDecimal

actual fun decimalRoundTrip(value: BigDecimal): BigDecimal =
    FfiConverterTypeDecimal.lift(FfiConverterTypeDecimal.lower(value))
