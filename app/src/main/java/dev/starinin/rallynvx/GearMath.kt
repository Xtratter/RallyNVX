package dev.starinin.rallynvx

/** The gear indicator's text and the revs' bar. */
object GearMath {
    const val LIMIT = 7800f
    fun label(gear: Int): String = when { gear < 0 -> "R"; gear == 0 -> "N"; else -> gear.toString() }
    fun mode(manual: Boolean, en: Boolean): String = if (manual) (if (en) "manual" else "МКПП") else (if (en) "auto" else "АКПП")
    /** The share of the bar the revs fill: from 0 at a standstill to 1 at the limiter. */
    fun revFraction(rpm: Float): Float = if (rpm.isFinite()) (rpm / LIMIT).coerceIn(0f, 1f) else 0f
    /** The clutch pedal from the finger's place on the pad: 0 at the pad's top (up), 1 at its bottom (down)... the pedal is pressed by touching it, harder lower. */
    fun clutchFromTouch(y: Float, height: Float): Float = if (height <= 1f) 1f else ((y / height) * 1.25f - 0.1f).coerceIn(0f, 1f)
}
