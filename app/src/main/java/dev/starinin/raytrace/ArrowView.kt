package dev.starinin.raytrace

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import io.github.xtratter.uikit.M3

/** Compass arrow to the mission target: 0 = up (ahead), positive angle = clockwise (target on the right). */
class ArrowView(ctx: Context) : View(ctx) {
    private val dp = ctx.resources.displayMetrics.density
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = M3.primary }
    private val path = Path()
    private var angle = 0f

    fun setAngle(a: Float) { angle = a; invalidate() }

    override fun onMeasure(w: Int, h: Int) { val s = (56f * dp).toInt(); setMeasuredDimension(s, s) }

    override fun onDraw(c: Canvas) {
        val cx = width / 2f; val cy = height / 2f; val r = width * 0.42f
        c.save()
        c.rotate(Math.toDegrees(angle.toDouble()).toFloat(), cx, cy)
        path.reset()
        path.moveTo(cx, cy - r); path.lineTo(cx + r * 0.65f, cy + r * 0.7f); path.lineTo(cx, cy + r * 0.3f); path.lineTo(cx - r * 0.65f, cy + r * 0.7f); path.close()
        c.drawPath(path, paint)
        c.restore()
    }
}
