package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.view.View
import io.github.xtratter.uikit.M3

/** The gear indicator: the gear in big, the mode (automatic or manual) and a bar of the engine's revs with its red zone; a stalled engine blinks. */
class GearView(ctx: Context) : View(ctx) {
    private val dp = ctx.resources.displayMetrics.density
    private var gear = 1; private var rpm = 0f; private var manual = false; private var stalled = false
    private val pad = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x66000000 }
    private val big = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFFFFFFF.toInt(); textSize = 40 * dp; typeface = Typeface.create("sans-serif-medium", Typeface.BOLD); textAlign = Paint.Align.CENTER }
    private val small = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFD0D8E4.toInt(); textSize = 12 * dp; textAlign = Paint.Align.CENTER }
    private val bar = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }

    init { alpha = 0.95f }
    override fun onMeasure(wm: Int, hm: Int) { setMeasuredDimension((96f * dp).toInt(), (84f * dp).toInt()) }

    fun update(gear: Int, rpm: Float, manual: Boolean, stalled: Boolean) {
        if (gear != this.gear || manual != this.manual || stalled != this.stalled || Math.abs(rpm - this.rpm) > 20f) { this.gear = gear; this.rpm = rpm; this.manual = manual; this.stalled = stalled; invalidate() }
    }

    override fun onDraw(c: Canvas) {
        val r = RectF(0f, 0f, width.toFloat(), height.toFloat())
        c.drawRoundRect(r, 18f * dp, 18f * dp, pad)
        val label = GearMath.label(gear)
        big.color = if (stalled && (System.currentTimeMillis() / 400) % 2L == 0L) 0xFFFF6B5E.toInt() else 0xFFFFFFFF.toInt()
        c.drawText(label, width / 2f, height * 0.55f, big)
        c.drawText(GearMath.mode(manual, t("en", "ru") == "en") + if (stalled) " · " + t("stalled", "заглох") else "", width / 2f, height * 0.72f, small)
        // the revs: a bar from the idle to the limiter; the last fifth is the red zone
        val x0 = 10f * dp; val x1 = width - 10f * dp; val y = height - 10f * dp
        bar.color = 0x55FFFFFF; c.drawRoundRect(RectF(x0, y - 4f * dp, x1, y), 2f * dp, 2f * dp, bar)
        val f = GearMath.revFraction(rpm)
        bar.color = if (f > 0.85f) 0xFFFF5A4D.toInt() else M3.primary
        c.drawRoundRect(RectF(x0, y - 4f * dp, x0 + (x1 - x0) * f, y), 2f * dp, 2f * dp, bar)
        if (stalled) postInvalidateOnAnimation()
    }
}
