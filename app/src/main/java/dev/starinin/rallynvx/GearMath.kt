package dev.starinin.rallynvx

/** The gear indicator's text and the revs' bar. */
object GearMath {
    const val LIMIT = 8000f
    /** The red zone of the tachometer starts here. */
    const val REDLINE = 7000f
    /** The dials: they start at the lower left (150 degrees clockwise from 3 o'clock) and sweep 240 degrees. */
    const val DIAL_START = 150f
    const val DIAL_SWEEP = 240f
    const val BOOST_MAX = 0.6f
    /** The share of the boost arc a pressure (bar over the atmosphere) fills. */
    fun boostFraction(bar: Float): Float = if (bar.isFinite()) (bar / BOOST_MAX).coerceIn(0f, 1f) else 0f
    /** The manual box types (0 sequential, the real car's; 1 H-pattern with a clutch) and the automatic's characters (0 comfort, 1 sport, 2 race). */
    const val BOXES = 2
    const val PROFILES = 3
    fun boxName(i: Int, en: Boolean): String = if (i == 1) (if (en) "H-pattern" else "H-коробка") else (if (en) "Sequential" else "Секвентальная")
    fun profileName(i: Int, en: Boolean): String = when (i) { 0 -> if (en) "Comfort" else "Комфорт"; 2 -> if (en) "Race" else "Гоночный"; else -> if (en) "Sport" else "Спорт" }
    /** The next value of a cycle of `n`; a value out of range starts it again. */
    fun next(i: Int, n: Int): Int = if (i < 0 || i >= n - 1) 0 else i + 1
    fun label(gear: Int): String = when { gear < 0 -> "R"; gear == 0 -> "N"; else -> gear.toString() }
    fun mode(manual: Boolean, en: Boolean): String = if (manual) (if (en) "manual" else "МКПП") else (if (en) "auto" else "АКПП")
    /** The share of the bar the revs fill: from 0 at a standstill to 1 at the limiter. */
    fun revFraction(rpm: Float): Float = if (rpm.isFinite()) (rpm / LIMIT).coerceIn(0f, 1f) else 0f
    /** The clutch pedal from the finger's place on the pad: 0 at the pad's top (up), 1 at its bottom (down)... the pedal is pressed by touching it, harder lower. */
    fun clutchFromTouch(y: Float, height: Float): Float = if (height <= 1f) 1f else ((y / height) * 1.25f - 0.1f).coerceIn(0f, 1f)
}
