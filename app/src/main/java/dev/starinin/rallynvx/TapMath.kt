package dev.starinin.rallynvx

/** Double-tap detection for the free space of the screen: two quick, short taps near each other. Times in milliseconds, distances in pixels. */
class DoubleTap(private val slop: Float, private val maxTap: Long = 250L, private val gap: Long = 350L) {
    private var downT = 0L; private var downX = 0f; private var downY = 0f
    private var lastT = -1L; private var lastX = 0f; private var lastY = 0f

    fun down(t: Long, x: Float, y: Float) { downT = t; downX = x; downY = y }

    /** The finger lifts at (x, y); `true` when this tap completes a double tap. A drag or a long press is not a tap. */
    fun up(t: Long, x: Float, y: Float): Boolean {
        val tap = t - downT <= maxTap && Math.hypot((x - downX).toDouble(), (y - downY).toDouble()) <= slop
        if (!tap) { lastT = -1L; return false }
        val second = lastT >= 0 && t - lastT <= gap && Math.hypot((x - lastX).toDouble(), (y - lastY).toDouble()) <= slop * 3
        if (second) { lastT = -1L; return true }
        lastT = t; lastX = x; lastY = y
        return false
    }

    fun cancel() { lastT = -1L }
}
