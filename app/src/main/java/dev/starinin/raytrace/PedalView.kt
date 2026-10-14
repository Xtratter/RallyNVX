package dev.starinin.raytrace

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.view.View
import io.github.xtratter.uikit.M3

/** A pedal: a rounded pad filled from the bottom by [pressure] (0..1) with a gas arrow or a brake square. */
class PedalView(ctx: Context, private val gas: Boolean, private val w: Float, private val h: Float) : View(ctx) {
    var pressure = 0f
        set(v) { if (v != field) { field = v; invalidate() } }
    private val dp = ctx.resources.displayMetrics.density
    private val pad = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x44000000 }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = M3.primary; alpha = 200 }
    private val icon = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFFFFFFF.toInt(); alpha = 230 }
    private val rim = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 3f * dp; color = M3.primary; alpha = 220 }

    override fun onMeasure(wm: Int, hm: Int) { setMeasuredDimension((w * dp).toInt(), (h * dp).toInt()) }

    override fun onDraw(c: Canvas) {
        val r = RectF(2f * dp, 2f * dp, width - 2f * dp, height - 2f * dp); val cr = 16f * dp
        c.drawRoundRect(r, cr, cr, pad)
        if (pressure > 0f) {
            c.save(); c.clipRect(0f, height * (1f - pressure), width.toFloat(), height.toFloat())
            c.drawRoundRect(r, cr, cr, fill); c.restore()
        }
        c.drawRoundRect(r, cr, cr, rim)
        val cx = width / 2f; val cy = height / 2f; val s = width * 0.22f
        if (gas) {
            val p = Path().apply { moveTo(cx, cy - s); lineTo(cx + s, cy + s * 0.6f); lineTo(cx - s, cy + s * 0.6f); close() }
            c.drawPath(p, icon)
        } else c.drawRect(cx - s * 0.8f, cy - s * 0.8f, cx + s * 0.8f, cy + s * 0.8f, icon)
    }
}
