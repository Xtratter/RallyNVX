package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Shader
import android.view.View
import io.github.xtratter.uikit.M3

/**
 * A pedal drawn as a pedal: a metal plate with a rubber pad that sinks as it is pressed ([pressure] 0..1) and takes the accent colour from the bottom up. The gas is the tall
 * organ pedal with a hinge and ridges across, the brake and the clutch the wide rubber pads (the clutch with a ring for the disc).
 */
class PedalView(ctx: Context, private val kind: Int, private val w: Float, private val h: Float) : View(ctx) {
    companion object { const val GAS = 0; const val BRAKE = 1; const val CLUTCH = 2 }
    var pressure = 0f
        set(v) { if (v != field) { field = v; invalidate() } }
    private val dp = ctx.resources.displayMetrics.density
    private val fillP = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val line = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val accent = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xFFEDEFF2.toInt(); alpha = 215 }

    override fun onMeasure(wm: Int, hm: Int) { setMeasuredDimension((w * dp).toInt(), (h * dp).toInt()) }

    override fun onDraw(c: Canvas) {
        val W = width.toFloat(); val H = height.toFloat()
        // the metal plate (the arm), with a bright edge
        val plate = RectF(W * 0.10f, H * 0.03f, W * 0.90f, H * 0.97f)
        fillP.shader = LinearGradient(0f, plate.top, 0f, plate.bottom, 0xFF515861.toInt(), 0xFF22262B.toInt(), Shader.TileMode.CLAMP)
        c.drawRoundRect(plate, 12f * dp, 12f * dp, fillP)
        line.strokeWidth = 1.6f * dp; line.color = 0xAAB8C2CE.toInt(); c.drawRoundRect(plate, 12f * dp, 12f * dp, line)
        // the rubber pad sinks with the pressure
        val sink = pressure * 7f * dp
        val pad = RectF(plate.left + 6f * dp, plate.top + 8f * dp + sink, plate.right - 6f * dp, plate.bottom - (if (kind == GAS) 22f else 8f) * dp + sink * 0.4f)
        fillP.shader = LinearGradient(0f, pad.top, 0f, pad.bottom, 0xFF34383E.toInt(), 0xFF0F1114.toInt(), Shader.TileMode.CLAMP)
        c.drawRoundRect(pad, 8f * dp, 8f * dp, fillP)
        fillP.shader = null
        // the ribs of the rubber
        line.strokeWidth = 2.2f * dp; line.color = 0x55FFFFFF
        val step = if (kind == GAS) 13f * dp else 16f * dp
        var y = pad.top + step * 0.7f
        while (y < pad.bottom - 4f * dp) { c.drawLine(pad.left + 8f * dp, y, pad.right - 8f * dp, y, line); y += step }
        if (kind == CLUTCH) { line.strokeWidth = 3f * dp; line.color = 0xAAFFFFFF.toInt(); c.drawCircle(pad.centerX(), pad.centerY(), pad.width() * 0.2f, line) }
        // the pressure: the accent comes up from the bottom of the pad
        if (pressure > 0.02f) {
            c.save(); c.clipRect(pad.left, pad.bottom - pad.height() * pressure, pad.right, pad.bottom)
            accent.alpha = 150; c.drawRoundRect(pad, 8f * dp, 8f * dp, accent); c.restore()
        }
        // the hinge of the organ pedal
        if (kind == GAS) { fillP.color = 0xFF9AA4B0.toInt(); c.drawCircle(W / 2f, H - 12f * dp, 6f * dp, fillP); fillP.color = 0xFF2A2E34.toInt(); c.drawCircle(W / 2f, H - 12f * dp, 2.6f * dp, fillP) }
    }
}
