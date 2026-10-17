package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.view.View
import io.github.xtratter.uikit.M3

/**
 * The instrument cluster: a tachometer (0 to 8000 rpm, the red zone from 7000, a needle), the boost gauge on an inner arc (0 to 0.6 bar), the gear in big at the centre with the mode
 * under it and the boost in bar below; a stalled engine blinks the gear red.
 */
class GearView(ctx: Context) : View(ctx) {
    private val dp = ctx.resources.displayMetrics.density
    private var gear = 1; private var rpm = 0f; private var manual = false; private var stalled = false; private var boost = 0f
    private val pad = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x88000000.toInt() }
    private val arc = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeCap = Paint.Cap.ROUND }
    private val tick = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; color = 0xCCFFFFFF.toInt() }
    private val needle = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeCap = Paint.Cap.ROUND; color = 0xFFFF5A4D.toInt() }
    private val big = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFFFFFFF.toInt(); textSize = 40 * dp; typeface = Typeface.create("sans-serif-medium", Typeface.BOLD); textAlign = Paint.Align.CENTER }
    private val small = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFD0D8E4.toInt(); textSize = 11 * dp; textAlign = Paint.Align.CENTER }
    private val num = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFE8EEF6.toInt(); textSize = 10 * dp; textAlign = Paint.Align.CENTER }

    override fun onMeasure(wm: Int, hm: Int) { setMeasuredDimension((156f * dp).toInt(), (156f * dp).toInt()) }

    fun update(gear: Int, rpm: Float, manual: Boolean, stalled: Boolean, boostBar: Float) {
        if (gear != this.gear || manual != this.manual || stalled != this.stalled || Math.abs(rpm - this.rpm) > 15f || Math.abs(boostBar - this.boost) > 0.01f) {
            this.gear = gear; this.rpm = rpm; this.manual = manual; this.stalled = stalled; this.boost = boostBar; invalidate()
        }
    }

    override fun onDraw(c: Canvas) {
        val cx = width / 2f; val cy = height / 2f; val r = width / 2f - 4f * dp
        c.drawCircle(cx, cy, r + 2f * dp, pad)
        val oval = RectF(cx - r * 0.86f, cy - r * 0.86f, cx + r * 0.86f, cy + r * 0.86f)
        // the dial: 240 degrees from the lower left round the top to the lower right
        val start = GearMath.DIAL_START; val sweep = GearMath.DIAL_SWEEP
        arc.strokeWidth = 5f * dp; arc.color = 0x55FFFFFF
        c.drawArc(oval, start, sweep, false, arc)
        arc.color = 0xFFFF5A4D.toInt()
        c.drawArc(oval, start + sweep * GearMath.REDLINE / GearMath.LIMIT, sweep * (1f - GearMath.REDLINE / GearMath.LIMIT), false, arc)
        arc.color = M3.primary
        c.drawArc(oval, start, sweep * GearMath.revFraction(rpm).coerceAtMost(GearMath.REDLINE / GearMath.LIMIT), false, arc)
        // the ticks and the numbers of the thousands
        for (k in 0..8) {
            val a = Math.toRadians((start + sweep * k / 8f).toDouble()); val ca = Math.cos(a).toFloat(); val sa = Math.sin(a).toFloat()
            tick.strokeWidth = if (k % 2 == 0) 2.2f * dp else 1.2f * dp
            c.drawLine(cx + ca * r * 0.74f, cy + sa * r * 0.74f, cx + ca * r * 0.82f, cy + sa * r * 0.82f, tick)
            if (k % 2 == 0) c.drawText(k.toString(), cx + ca * r * 0.62f, cy + sa * r * 0.62f + 4f * dp, num)
        }
        // the needle
        val na = Math.toRadians((start + sweep * GearMath.revFraction(rpm)).toDouble())
        needle.strokeWidth = 2.6f * dp
        c.drawLine(cx + Math.cos(na).toFloat() * r * 0.30f, cy + Math.sin(na).toFloat() * r * 0.30f, cx + Math.cos(na).toFloat() * r * 0.80f, cy + Math.sin(na).toFloat() * r * 0.80f, needle)
        // the boost: an inner arc under the numbers' ring
        val inner = RectF(cx - r * 0.42f, cy - r * 0.42f, cx + r * 0.42f, cy + r * 0.42f)
        arc.strokeWidth = 3f * dp; arc.color = 0x44FFFFFF; c.drawArc(inner, start, sweep, false, arc)
        arc.color = 0xFF7CC4FF.toInt(); c.drawArc(inner, start, sweep * GearMath.boostFraction(boost), false, arc)
        big.color = if (stalled && (System.currentTimeMillis() / 400) % 2L == 0L) 0xFFFF6B5E.toInt() else 0xFFFFFFFF.toInt()
        big.textSize = 34f * dp
        c.drawText(GearMath.label(gear), cx, cy + 9f * dp, big)
        c.drawText(GearMath.mode(manual, t("en", "ru") == "en") + if (stalled) " · " + t("stalled", "заглох") else "", cx, cy + r * 0.50f, small)
        c.drawText(String.format(java.util.Locale.US, "%.2f bar", boost.coerceAtLeast(0f)), cx, cy + r * 0.50f + 12f * dp, small)
        if (stalled) postInvalidateOnAnimation()
    }
}
