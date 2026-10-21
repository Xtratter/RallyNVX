package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.view.View
import io.github.xtratter.uikit.M3

/** A gear shift button of the manual: a rounded pad with an up or down triangle; [held] lights it while a finger is on it. */
class ShiftButtonView(ctx: Context, private val up: Boolean) : View(ctx) {
    var held = false
        set(v) { if (v != field) { field = v; invalidate() } }
    private val dp = ctx.resources.displayMetrics.density
    private val pad = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0x44000000 }
    private val litPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFFFFFFF.toInt(); alpha = 70 }
    private val icon = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFFFFFFF.toInt(); alpha = 230 }
    private val rim = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 1.5f * dp; color = 0xFFFFFFFF.toInt(); alpha = 150 }

    override fun onMeasure(wm: Int, hm: Int) { setMeasuredDimension((84f * dp).toInt(), (56f * dp).toInt()) }

    override fun onDraw(c: Canvas) {
        val r = RectF(2f * dp, 2f * dp, width - 2f * dp, height - 2f * dp); val cr = 10f * dp
        c.drawRoundRect(r, cr, cr, if (held) litPaint else pad)
        c.drawRoundRect(r, cr, cr, rim)
        val cx = width / 2f; val cy = height / 2f; val s = height * 0.2f
        val p = Path().apply { if (up) { moveTo(cx, cy - s); lineTo(cx + s * 1.3f, cy + s); lineTo(cx - s * 1.3f, cy + s) } else { moveTo(cx, cy + s); lineTo(cx + s * 1.3f, cy - s); lineTo(cx - s * 1.3f, cy - s) }; close() }
        c.drawPath(p, icon)
    }
}
