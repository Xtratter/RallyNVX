package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.view.View
import io.github.xtratter.uikit.M3

/** The on-screen steering wheel: a ring with three spokes that rotates clockwise-positive by [angle] degrees. */
class WheelView(ctx: Context) : View(ctx) {
    var angle = 0f
        set(v) { if (v != field) { field = v; invalidate() } }
    private val dp = ctx.resources.displayMetrics.density
    private val ring = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 4f * dp; color = 0xFFFFFFFF.toInt(); alpha = 170 }
    private val spoke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 3f * dp; strokeCap = Paint.Cap.ROUND; color = 0xFFFFFFFF.toInt(); alpha = 170 }
    private val hub = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFFFFFFF.toInt(); alpha = 200 }
    private val mark = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 10f * dp; strokeCap = Paint.Cap.ROUND; color = 0xFFFFFFFF.toInt() }
    private val back = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x33000000 }

    override fun onMeasure(w: Int, h: Int) { val s = (190f * dp).toInt(); setMeasuredDimension(s, s) }

    override fun onDraw(c: Canvas) {
        val cx = width / 2f; val cy = height / 2f; val r = width / 2f - ring.strokeWidth
        c.drawCircle(cx, cy, r + ring.strokeWidth / 2f, back)
        c.save(); c.rotate(angle, cx, cy)
        c.drawCircle(cx, cy, r, ring)
        c.drawLine(cx, cy, cx - r, cy, spoke); c.drawLine(cx, cy, cx + r, cy, spoke); c.drawLine(cx, cy, cx, cy + r, spoke)
        c.drawCircle(cx, cy, 16f * dp, hub)
        c.drawLine(cx, cy - r - ring.strokeWidth * 0.4f, cx, cy - r + ring.strokeWidth * 0.4f, mark)
        c.restore()
    }
}
