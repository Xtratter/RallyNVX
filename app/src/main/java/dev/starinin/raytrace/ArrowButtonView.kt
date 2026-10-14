package dev.starinin.raytrace

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.view.View
import io.github.xtratter.uikit.M3

/** A steering button with a left or right arrow; filled while pressed. */
class ArrowButtonView(ctx: Context, private val left: Boolean) : View(ctx) {
    var down = false
        set(v) { if (v != field) { field = v; invalidate() } }
    private val dp = ctx.resources.displayMetrics.density
    private val pad = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x44000000 }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = M3.primary; alpha = 200 }
    private val icon = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFFFFFFF.toInt(); alpha = 230 }
    private val rim = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 3f * dp; color = M3.primary; alpha = 220 }

    override fun onMeasure(w: Int, h: Int) { val s = (96f * dp).toInt(); setMeasuredDimension(s, s) }

    override fun onDraw(c: Canvas) {
        val r = RectF(2f * dp, 2f * dp, width - 2f * dp, height - 2f * dp); val cr = 22f * dp
        c.drawRoundRect(r, cr, cr, if (down) fill else pad)
        c.drawRoundRect(r, cr, cr, rim)
        val cx = width / 2f; val cy = height / 2f; val s = width * 0.2f; val d = if (left) -1f else 1f
        val p = Path().apply { moveTo(cx + d * s, cy); lineTo(cx - d * s, cy - s * 1.1f); lineTo(cx - d * s, cy + s * 1.1f); close() }
        c.drawPath(p, icon)
    }
}
