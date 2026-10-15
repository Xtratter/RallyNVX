package dev.starinin.rallynvx

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
    /** A finger dragging on free space: (dx, dy) in pixels since the last event; `onCameraUp` when it lifts. */
    var onCamera: (Float, Float) -> Unit = { _, _ -> }
    var onCameraUp: () -> Unit = {}
    /** Two fingers on free space: the span between them changed by `ratio` (new / old) and their middle moved by `dy` pixels (negative = up). */
    var onCameraPinch: (Float, Float) -> Unit = { _, _ -> }
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
    private val leftBtn = ArrowButtonView(ctx, true)
    private val rightBtn = ArrowButtonView(ctx, false)
    private val buttons = DriveMath.Buttons()
    private var analog = false
    private var leftId = -1; private var rightId = -1
    private var buttonsMode = true
    private var wheelMode = false
    private var tiltMode = false
    private var sens = 1
    private var wheelId = -1; private var gasId = -1; private var brakeId = -1
    private var camId = -1; private var camX = 0f; private var camY = 0f
    /** The second free-space finger: with the first it pinches (distance) and moves up or down (height). */
    private var cam2Id = -1; private var cam2X = 0f; private var cam2Y = 0f
    private var sent = floatArrayOf(Float.NaN, 0f, 0f, 0f)
    private var lastNs = 0L
    private val loop = object : Runnable { override fun run() { frame(); postOnAnimation(this) } }

    init {
        addView(wheelView); addView(leftBtn); addView(rightBtn); addView(gasView); addView(brakeView)
        configure(Controls.MODE_BUTTONS, 1, false, false)
    }

    /** Applies the preferences: steering mode, sensitivity and which side the pedals are on. */
    fun configure(mode: Int, sens: Int, pedalsLeft: Boolean, analogPedals: Boolean) {
        buttonsMode = mode == Controls.MODE_BUTTONS; wheelMode = mode == Controls.MODE_WHEEL; tiltMode = mode == Controls.MODE_TILT
        this.sens = sens; analog = analogPedals
        val m = (28f * dp).toInt()
        val pedalGrav = if (pedalsLeft) Gravity.BOTTOM or Gravity.START else Gravity.BOTTOM or Gravity.END
        val wheelGrav = if (pedalsLeft) Gravity.BOTTOM or Gravity.END else Gravity.BOTTOM or Gravity.START
        fun lp(g: Int, along: Int) = LayoutParams(-2, -2, g).apply {
            if (g and Gravity.END == Gravity.END) setMargins(0, 0, along, m) else setMargins(along, 0, 0, m)
        }
        wheelView.layoutParams = lp(wheelGrav, m)
        leftBtn.layoutParams = lp(wheelGrav, m)
        rightBtn.layoutParams = lp(wheelGrav, m + (112f * dp).toInt())
        leftBtn.visibility = if (buttonsMode) VISIBLE else GONE; rightBtn.visibility = if (buttonsMode) VISIBLE else GONE
        gasView.layoutParams = lp(pedalGrav, m)
        brakeView.layoutParams = lp(pedalGrav, m + (100f * dp).toInt())
        wheelView.visibility = if (buttonsMode) GONE else VISIBLE
        wheelView.alpha = if (tiltMode) 0.55f else 1f   // in tilt mode the wheel only shows the steering
        requestLayout()
    }

    fun calibrate() { tilt.calibrate(tiltRoll) }

    override fun onAttachedToWindow() { super.onAttachedToWindow(); lastNs = 0L; postOnAnimation(loop) }
    override fun onDetachedFromWindow() { removeCallbacks(loop); super.onDetachedFromWindow() }

    /** Lets go of everything (activity pause, panels): no stuck throttle, brake or steer. */
    fun release() {
        wheelId = -1; gasId = -1; brakeId = -1; leftId = -1; rightId = -1
        if (camId >= 0) { camId = -1; onCameraUp() }
        cam2Id = -1
        wheel.up(); buttons.reset(); leftBtn.down = false; rightBtn.down = false; gasView.pressure = 0f; brakeView.pressure = 0f
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
        val steer = if (buttonsMode) buttons.step(dt, leftBtn.down, rightBtn.down, sens) else if (tiltMode) (if (tiltLive) tilt.update(tiltRoll, dt, sens) else 0f) else wheel.steer(sens)
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
    private fun pedalAt(v: PedalView, y: Float) = if (analog) DriveMath.pedal((y - v.top).coerceIn(0f, v.height.toFloat()), v.height.toFloat()) else 1f

    override fun dispatchTouchEvent(e: MotionEvent): Boolean {
        if (visibility != VISIBLE) return false
        val any = wheelId >= 0 || gasId >= 0 || brakeId >= 0 || leftId >= 0 || rightId >= 0 || camId >= 0 || cam2Id >= 0
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val k = e.actionIndex; val pid = e.getPointerId(k); val x = e.getX(k); val y = e.getY(k)
                when {
                    leftId < 0 && inView(leftBtn, x, y) -> { leftId = pid; leftBtn.down = true }
                    rightId < 0 && inView(rightBtn, x, y) -> { rightId = pid; rightBtn.down = true }
                    wheelId < 0 && wheelMode && onWheel(x, y) -> { wheelId = pid; if (!nearHub(x, y)) wheel.down(touchAngle(x, y)) else wheel.down(90f) }
                    gasId < 0 && inView(gasView, x, y) -> { gasId = pid; gasView.pressure = pedalAt(gasView, y) }
                    brakeId < 0 && inView(brakeView, x, y) -> { brakeId = pid; brakeView.pressure = pedalAt(brakeView, y) }
                    camId < 0 -> { camId = pid; camX = x; camY = y }   // free space: a camera drag (also while the other fingers drive)
                    cam2Id < 0 -> { cam2Id = pid; cam2X = x; cam2Y = y; onCameraUp() }   // a second free finger: a pinch, the orbit drag pauses
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
                        camId -> if (cam2Id >= 0) {
                            // two fingers: the change of the span between them (distance) and of their middle height (height)
                            val j = e.findPointerIndex(cam2Id)
                            if (j >= 0) {
                                val (x1, y1, x2, y2) = floatArrayOf(e.getX(i), e.getY(i), e.getX(j), e.getY(j))
                                val old = hypot(camX - cam2X, camY - cam2Y); val now = hypot(x1 - x2, y1 - y2)
                                val ratio = if (old > 20f) now / old else 1f
                                val dy = ((y1 + y2) - (camY + cam2Y)) / 2f
                                if (ratio != 1f || dy != 0f) onCameraPinch(ratio, dy)
                                camX = x1; camY = y1; cam2X = x2; cam2Y = y2
                            }
                        } else { onCamera(e.getX(i) - camX, e.getY(i) - camY); camX = e.getX(i); camY = e.getY(i) }
                    }
                }
                report()
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val pid = e.getPointerId(e.actionIndex)
                if (pid == wheelId) { wheelId = -1; wheel.up() }
                if (pid == leftId) { leftId = -1; leftBtn.down = false }
                if (pid == rightId) { rightId = -1; rightBtn.down = false }
                if (pid == gasId) { gasId = -1; gasView.pressure = 0f }
                if (pid == brakeId) { brakeId = -1; brakeView.pressure = 0f }
                if (pid == cam2Id) { cam2Id = -1 }
                if (pid == camId) {
                    if (cam2Id >= 0) { camId = cam2Id; camX = cam2X; camY = cam2Y; cam2Id = -1 }   // the other finger carries on as a single drag
                    else { camId = -1; onCameraUp() }
                }
                report()
            }
            MotionEvent.ACTION_CANCEL -> release()
        }
        return any || wheelId >= 0 || gasId >= 0 || brakeId >= 0 || leftId >= 0 || rightId >= 0 || camId >= 0 || e.actionMasked == MotionEvent.ACTION_UP
    }
}
