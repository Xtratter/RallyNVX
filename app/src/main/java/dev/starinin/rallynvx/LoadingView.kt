package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.view.View

/** The start-up screen over the black surface: the title, a progress bar and what is loading. */
class LoadingView(ctx: Context) : View(ctx) {
    private var target = 0f
    private var shown = 0f
    private var text = ""
    private var last = 0L
    private var fading = false
    private val dp = resources.displayMetrics.density
    private val title = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFE3E8EF.toInt(); textSize = 28 * dp; typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL); textAlign = Paint.Align.CENTER }
    private val sub = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFA6B0BD.toInt(); textSize = 14 * dp; textAlign = Paint.Align.CENTER }
    private val track = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0x33FFFFFF }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFA8C8FF.toInt() }

    init { setBackgroundColor(0xFF0E1218.toInt()); isClickable = true }

    /** The loading starts again (the game restarts after the screen came back). */
    fun restart() { animate().cancel(); fading = false; target = 0f; shown = 0f; last = 0L; text = ""; visibility = VISIBLE; alpha = 1f; invalidate() }

    fun report(packed: Int) {
        target = LoadMath.percent(packed) / 100f
        text = LoadMath.label(packed, t("en", "ru") == "en") + " · " + LoadMath.percent(packed) + " %"
        if (LoadMath.done(packed) && visibility == VISIBLE && !fading) {   // once: a restarted animation never ends, and the clickable view would keep eating the touches
            fading = true
            animate().alpha(0f).setDuration(250).withEndAction { visibility = GONE }.start()
        }
        invalidate()
    }

    override fun onDraw(c: Canvas) {
        val now = System.nanoTime()
        val dt = if (last == 0L) 0f else ((now - last) / 1e9f).coerceIn(0f, 0.1f)
        last = now
        shown = LoadMath.ease(shown, target, dt)
        if (target < 1f && shown >= target - 0.001f) shown = minOf(shown + 0.008f * dt, target + 0.04f)   // a long step (the shaders) must not look frozen: the bar creeps
        val w = width.toFloat(); val h = height.toFloat()
        val bw = minOf(w * 0.6f, 360 * dp); val bh = 6 * dp
        val x = (w - bw) / 2; val y = h / 2 + 8 * dp
        c.drawText("RallyNVX", w / 2, y - 28 * dp, title)
        c.drawRoundRect(RectF(x, y, x + bw, y + bh), bh / 2, bh / 2, track)
        if (shown > 0.005f) c.drawRoundRect(RectF(x, y, x + bw * shown, y + bh), bh / 2, bh / 2, fill)
        c.drawText(text, w / 2, y + bh + 26 * dp, sub)
        if (visibility == VISIBLE && (shown < target - 0.001f || target < 1f)) postInvalidateOnAnimation()
    }
}
