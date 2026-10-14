package dev.starinin.raytrace

import android.content.Context
import android.view.Gravity
import android.view.MotionEvent
import android.widget.FrameLayout
import kotlin.math.hypot

/**
 * Car controls overlay: steering wheel (or the tilt indicator), gas and brake pedals. Each control follows its own pointer,
 * touches elsewhere fall through. Emits (throttle, steer, brake, brakeAmt) whenever something changes.
 */
class DriveControls(ctx: Context) : FrameLayout(ctx) {
    var onChange: (Float, Float, Boolean, Float) -> Unit = { _, _, _, _ -> }
    /** Current speed in km/h (the brake pedal reverses when nearly stopped). */
    var speedKmh = 0f
    /** Roll angle of the phone in degrees (tilt mode), fed by the activity. */
    var tiltRoll = 0f
    /** True while the tilt sensor delivers values (before the first value and after the sensor stops, tilt steers nothing). */
    var tiltLive = false
        set(v) { field = v; if (!v) tilt.reset() }
    /** True while the activity is paused: the loop idles and nothing is sent. */
    var paused = false
        set(v) { field = v; if (v) { release() } else lastNs = 0L }

    private val dp = ctx.resources.displayMetrics.density
    private val wheel = DriveMath.Wheel()
    private val tilt = DriveMath.Tilt()
    private val wheelView = WheelView(ctx)
    private val gasView = PedalView(ctx, true, 84f, 170f)
    private val brakeView = PedalView(ctx, false, 84f, 120f)
    private var tiltMode = false
    private var sens = 1
    private var wheelId = -1; private var gasId = -1; private var brakeId = -1
    private var sent = floatArrayOf(Float.NaN, 0f, 0f, 0f)
    private var lastNs = 0L
    private val loop = object : Runnable { override fun run() { frame(); postOnAnimation(this) } }

    init {
        addView(wheelView); addView(gasView); addView(brakeView)
        configure(Controls.MODE_WHEEL, 1, false)
    }

    /** Applies the preferences: steering mode, sensitivity and which side the pedals are on. */
    fun configure(mode: Int, sens: Int, pedalsLeft: Boolean) {
        tiltMode = mode == Controls.MODE_TILT; this.sens = sens
        val m = (28f * dp).toInt()
        val pedalGrav = if (pedalsLeft) Gravity.BOTTOM or Gravity.START else Gravity.BOTTOM or Gravity.END
        val wheelGrav = if (pedalsLeft) Gravity.BOTTOM or Gravity.END else Gravity.BOTTOM or Gravity.START
        fun lp(g: Int, along: Int) = LayoutParams(-2, -2, g).apply {
            if (g and Gravity.END == Gravity.END) setMargins(0, 0, along, m) else setMargins(along, 0, 0, m)
        }
        wheelView.layoutParams = lp(wheelGrav, m)
        gasView.layoutParams = lp(pedalGrav, m)
        brakeView.layoutParams = lp(pedalGrav, m + (100f * dp).toInt())
        wheelView.visibility = VISIBLE
        wheelView.alpha = if (tiltMode) 0.55f else 1f   // in tilt mode the wheel only shows the steering
        requestLayout()
    }

    fun calibrate() { tilt.calibrate(tiltRoll) }

    override fun onAttachedToWindow() { super.onAttachedToWindow(); lastNs = 0L; postOnAnimation(loop) }
    override fun onDetachedFromWindow() { removeCallbacks(loop); super.onDetachedFromWindow() }

    /** Lets go of everything (activity pause, panels): no stuck throttle, brake or steer. */
    fun release() {
        wheelId = -1; gasId = -1; brakeId = -1
        wheel.up(); gasView.pressure = 0f; brakeView.pressure = 0f
        report()
    }

    private fun frame() {
        if (paused) return
        val now = System.nanoTime()
        val dt = if (lastNs == 0L) 0.016f else ((now - lastNs) / 1e9f).coerceIn(0f, 0.1f)
        lastNs = now
        wheel.step(dt)
        if (!tiltMode) wheelView.angle = wheel.angle
        report(dt)
    }

    private fun report(dt: Float = 0f) {
        val steer = if (tiltMode) (if (tiltLive) tilt.update(tiltRoll, dt, sens) else 0f) else wheel.steer(sens)
        if (tiltMode) wheelView.angle = steer * DriveMath.WHEEL_MAX
        val cmd = DriveMath.brakeMap(brakeView.pressure, speedKmh)
        val throttle = (gasView.pressure + cmd.throttle).coerceIn(-1f, 1f)
        val out = floatArrayOf(throttle, steer, if (cmd.brake) 1f else 0f, cmd.amt)
        if (sent[0].isNaN() || (0..3).any { kotlin.math.abs(out[it] - sent[it]) > 0.004f }) {
            sent = out
            onChange(throttle, steer, cmd.brake, cmd.amt)
        }
    }

    private fun inView(v: android.view.View, x: Float, y: Float) = v.visibility == VISIBLE && x >= v.left && x <= v.right && y >= v.top && y <= v.bottom
    private fun onWheel(x: Float, y: Float): Boolean {
        if (wheelView.visibility != VISIBLE || wheelView.width == 0) return false
        val r = wheelView.width / 2f * 1.1f
        return hypot(x - (wheelView.left + wheelView.width / 2f), y - (wheelView.top + wheelView.height / 2f)) <= r
    }
    /** Within this radius of the hub the angle is too jumpy to follow (a finger passing the centre must not snap the wheel). */
    private fun nearHub(x: Float, y: Float) = hypot(x - (wheelView.left + wheelView.width / 2f), y - (wheelView.top + wheelView.height / 2f)) < 28f * dp
    private fun touchAngle(x: Float, y: Float) = DriveMath.wheelAngleDeg(x, y, wheelView.left + wheelView.width / 2f, wheelView.top + wheelView.height / 2f)
    private fun pedalAt(v: PedalView, y: Float) = DriveMath.pedal((y - v.top).coerceIn(0f, v.height.toFloat()), v.height.toFloat())

    override fun dispatchTouchEvent(e: MotionEvent): Boolean {
        if (visibility != VISIBLE) return false
        val any = wheelId >= 0 || gasId >= 0 || brakeId >= 0
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val k = e.actionIndex; val pid = e.getPointerId(k); val x = e.getX(k); val y = e.getY(k)
                when {
                    wheelId < 0 && !tiltMode && onWheel(x, y) -> { wheelId = pid; if (!nearHub(x, y)) wheel.down(touchAngle(x, y)) else wheel.down(90f) }
                    gasId < 0 && inView(gasView, x, y) -> { gasId = pid; gasView.pressure = pedalAt(gasView, y) }
                    brakeId < 0 && inView(brakeView, x, y) -> { brakeId = pid; brakeView.pressure = pedalAt(brakeView, y) }
                    else -> if (e.actionMasked == MotionEvent.ACTION_DOWN && !any) return false
                }
                report()
            }
            MotionEvent.ACTION_MOVE -> {
                for (i in 0 until e.pointerCount) {
                    val pid = e.getPointerId(i)
                    when (pid) {
                        wheelId -> { if (!nearHub(e.getX(i), e.getY(i))) wheel.move(touchAngle(e.getX(i), e.getY(i))); wheelView.angle = wheel.angle }
                        gasId -> gasView.pressure = pedalAt(gasView, e.getY(i))
                        brakeId -> brakeView.pressure = pedalAt(brakeView, e.getY(i))
                    }
                }
                report()
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val pid = e.getPointerId(e.actionIndex)
                if (pid == wheelId) { wheelId = -1; wheel.up() }
                if (pid == gasId) { gasId = -1; gasView.pressure = 0f }
                if (pid == brakeId) { brakeId = -1; brakeView.pressure = 0f }
                report()
            }
            MotionEvent.ACTION_CANCEL -> release()
        }
        return any || wheelId >= 0 || gasId >= 0 || brakeId >= 0 || e.actionMasked == MotionEvent.ACTION_UP
    }
}
