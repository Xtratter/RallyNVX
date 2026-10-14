package dev.starinin.raytrace

import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.exp
import kotlin.math.sign

/** Pure driving-input shaping (no Android): steering wheel, pedals, tilt, preference steps. */
object DriveMath {
    val SENS = floatArrayOf(0.7f, 1f, 1.4f)
    val VOLUME = floatArrayOf(0f, 0.25f, 0.5f, 0.75f, 1f)
    const val WHEEL_MAX = 120f        // degrees of lock either way
    const val WHEEL_RETURN = 400f     // degrees per second back to the centre
    const val REVERSE_KMH = 3f        // at or below this speed the brake pedal reverses

    fun cycle(i: Int, n: Int) = (i + 1) % n
    /** Default render mode by quality index: hybrid (1) on High (2), raster (0) otherwise. */
    fun defaultRender(quality: Int) = if (quality == 2) 1 else 0

    /** Angle of (x, y) around (cx, cy) in degrees with y pointing up on the screen (touch y grows downwards): right 0, up 90, left 180, down -90. */
    fun wheelAngleDeg(x: Float, y: Float, cx: Float, cy: Float): Float =
        Math.toDegrees(atan2((cy - y).toDouble(), (x - cx).toDouble())).toFloat()

    private fun wrap180(d: Float): Float { var a = d % 360f; if (a > 180f) a -= 360f; if (a < -180f) a += 360f; return a }

    /** The steering wheel: follows a finger around its centre (clockwise = positive = steer right), springs back on release. */
    class Wheel(val maxDeg: Float = WHEEL_MAX) {
        var angle = 0f; private set
        private var last: Float? = null
        val touching get() = last != null
        fun down(touchDeg: Float) { if (touchDeg.isFinite()) last = touchDeg }
        fun move(touchDeg: Float) {
            if (!touchDeg.isFinite()) return
            val l = last ?: run { last = touchDeg; return }
            angle = (angle - wrap180(touchDeg - l)).coerceIn(-maxDeg, maxDeg)
            last = touchDeg
        }
        fun up() { last = null }
        fun step(dt: Float) {
            if (touching || !dt.isFinite() || dt <= 0f) return
            val d = WHEEL_RETURN * dt
            angle = if (abs(angle) <= d) 0f else angle - sign(angle) * d
        }
        /** Steering command -1..1: a gentle curve with the sensitivity gain, saturating at full lock. */
        fun steer(sens: Int): Float {
            val x = angle / maxDeg
            return ((0.7f * x + 0.3f * x * abs(x)) * SENS[sens.coerceIn(0, SENS.size - 1)]).coerceIn(-1f, 1f)
        }
    }

    /** Pedal pressure from the touch height [y] inside a pedal of height [h] (top light, bottom full); 0 outside. */
    fun pedal(y: Float, h: Float): Float = if (!y.isFinite() || !h.isFinite() || h <= 0f || y < 0f || y > h) 0f else 0.15f + 0.85f * (y / h)

    /** What the brake pedal does: brake while moving, reverse (negative throttle) when nearly stopped. */
    data class BrakeCmd(val brake: Boolean, val amt: Float, val throttle: Float)
    fun brakeMap(pressure: Float, kmh: Float): BrakeCmd = when {
        !(pressure > 0f) -> BrakeCmd(false, 0f, 0f)
        kmh > REVERSE_KMH -> BrakeCmd(true, pressure.coerceIn(0.05f, 1f), 0f)
        else -> BrakeCmd(false, 0f, -pressure.coerceIn(0f, 1f))
    }

    /** Steering by two buttons: the value ramps towards full lock while a button is held and back to centre on release. */
    class Buttons {
        var value = 0f; private set
        fun step(dt: Float, left: Boolean, right: Boolean, sens: Int): Float {
            if (!dt.isFinite() || dt <= 0f) return value
            val target = (if (right) 1f else 0f) - (if (left) 1f else 0f)
            val rate = (if (target == 0f) 8f else 4f * SENS[sens.coerceIn(0, SENS.size - 1)]) * dt
            value = if (abs(target - value) <= rate) target else value + sign(target - value) * rate
            return value
        }
        fun reset() { value = 0f }
    }

    /** Tilt steering from the phone's roll angle (degrees, landscape): re-zeroable, dead zone, low-pass. */
    class Tilt {
        private var zero = 0f
        private var smooth = 0f
        fun calibrate(rollDeg: Float) { if (rollDeg.isFinite()) { zero = rollDeg; smooth = 0f } }
        /** Forgets the centre and the smoothing (the sensor was switched off). */
        fun reset() { zero = 0f; smooth = 0f }
        fun update(rollDeg: Float, dt: Float, sens: Int): Float {
            if (!rollDeg.isFinite() || !dt.isFinite() || dt < 0f) return 0f
            val a = 1f - exp(-dt / 0.08f)
            smooth += (wrap180(rollDeg - zero) - smooth) * a
            val mag = (abs(smooth) - 3f).coerceAtLeast(0f)
            return sign(smooth) * (mag / (27f / SENS[sens.coerceIn(0, SENS.size - 1)])).coerceIn(0f, 1f)
        }
    }
}
