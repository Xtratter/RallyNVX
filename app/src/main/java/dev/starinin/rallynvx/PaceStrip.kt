package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Typeface
import android.view.View
import kotlin.math.cos
import kotlin.math.sin

/** How a pace note looks as a pictogram: the colour by severity and how far the arrow bends (6 easy ... 1 sharp, 0 hairpin). */
object PaceStyle {
    const val GREEN = 0xFF43C463.toInt()
    const val YELLOW = 0xFFF2C94C.toInt()
    const val ORANGE = 0xFFF2994A.toInt()
    const val RED = 0xFFEB4D4B.toInt()
    const val GREY = 0xFFE0E0E0.toInt()

    fun color(sev: Int) = when { sev >= 4 -> GREEN; sev >= 2 -> YELLOW; sev == 1 -> ORANGE; else -> RED }

    /** The bend of the arrow in degrees from straight on; a hairpin is drawn as a U-turn instead. */
    fun bend(sev: Int) = when (sev) { 6 -> 12f; 5 -> 25f; 4 -> 40f; 3 -> 58f; 2 -> 80f; else -> 105f }

    /** A distance as spoken and shown: to 10 m under 100, to 50 m under 300, else to 100 m. */
    fun roundDistance(m: Float): Int {
        val step = if (m < 100f) 10f else if (m < 300f) 50f else 100f
        return (Math.round(m / step) * step).toInt()
    }
}

/** The strip of the next pace notes drawn as pictograms (circle, arrow, number), with the distances between them. */
class PaceStrip(ctx: Context) : View(ctx) {
    private val dp = ctx.resources.displayMetrics.density
    private val ring = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 3f * dp; strokeCap = Paint.Cap.ROUND }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = 0xB0161A22.toInt() }
    private val line = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeCap = Paint.Cap.ROUND; strokeJoin = Paint.Join.ROUND }
    private val solid = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val label = Paint(Paint.ANTI_ALIAS_FLAG).apply { typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD); textAlign = Paint.Align.CENTER }
    private var notes: List<FloatArray> = emptyList()

    /** `st` is the game's state array: four notes of 8 numbers from index 45 (see `Playback::upcoming`). */
    fun update(st: FloatArray) {
        val list = ArrayList<FloatArray>()
        for (i in 0 until 3) { val b = 45 + i * 8; if (b + 8 <= st.size && st[b] > 0.5f) list.add(st.copyOfRange(b, b + 8)) }
        if (list.size != notes.size || list.indices.any { !list[it].contentEquals(notes[it]) }) { notes = list; requestLayout(); invalidate() }
    }

    private fun items(): List<Item> {
        val out = ArrayList<Item>()
        for ((k, n) in notes.withIndex()) {
            val big = k == 0
            out.add(Item.Icon(n, big))
            if (n[3].toInt() and 8 != 0) { out.add(Item.Into); out.add(Item.Icon(floatArrayOf(1f, n[5], n[6], 0f, 0f, 0f, 0f, 0f), big)) }
            val gap = n[7]
            if (gap > 60f && k < notes.size - 1) out.add(Item.Gap(PaceStyle.roundDistance(gap)))
        }
        return out
    }

    private sealed class Item { class Icon(val n: FloatArray, val big: Boolean) : Item(); object Into : Item(); class Gap(val m: Int) : Item() }

    private fun width(i: Item) = when (i) { is Item.Icon -> (if (i.big) 50f else 40f) * dp; Item.Into -> 20f * dp; is Item.Gap -> 40f * dp }

    override fun onMeasure(w: Int, h: Int) {
        val total = items().sumOf { width(it).toDouble() }.toFloat() + 8f * dp
        setMeasuredDimension(total.toInt().coerceAtLeast(1), (54f * dp).toInt())
    }

    override fun onDraw(c: Canvas) {
        var x = 4f * dp
        val cy = height / 2f
        for (it in items()) {
            val w = width(it)
            when (it) {
                is Item.Icon -> icon(c, x + w / 2f, cy, (if (it.big) 23f else 18f) * dp, it.n)
                Item.Into -> { line.color = PaceStyle.GREY; line.strokeWidth = 2.5f * dp; c.drawLine(x + 3f * dp, cy, x + w - 4f * dp, cy, line); solid.color = PaceStyle.GREY
                    val p = Path().apply { moveTo(x + w - 2f * dp, cy); lineTo(x + w - 8f * dp, cy - 4f * dp); lineTo(x + w - 8f * dp, cy + 4f * dp); close() }; c.drawPath(p, solid) }
                is Item.Gap -> bubble(c, x + w / 2f, cy, it.m.toString())
            }
            x += w
        }
    }

    private fun bubble(c: Canvas, cx: Float, cy: Float, text: String) {
        solid.color = 0xE6FFFFFF.toInt()
        val r = RectF(cx - 17f * dp, cy - 10f * dp, cx + 17f * dp, cy + 10f * dp)
        c.drawRoundRect(r, 10f * dp, 10f * dp, solid)
        label.color = 0xFF202124.toInt(); label.textSize = 12f * dp
        c.drawText(text, cx, cy + 4.3f * dp, label)
    }

    private fun icon(c: Canvas, cx: Float, cy: Float, r: Float, n: FloatArray) {
        val kind = n[0].toInt(); val dir = if (n[1] > 0.5f) -1f else 1f   // a left turn bends the arrow to the left (negative x)
        val sev = n[2].toInt(); val flags = n[3].toInt()
        val col = when (kind) { 1 -> PaceStyle.color(sev); 2 -> if (flags and 16 != 0) PaceStyle.RED else PaceStyle.YELLOW; else -> PaceStyle.GREY }
        c.drawCircle(cx, cy, r, fill)
        ring.color = col; ring.strokeWidth = (if (r > 20f * dp) 3.2f else 2.6f) * dp
        c.drawCircle(cx, cy, r, ring)
        line.color = col; line.strokeWidth = r * 0.13f; solid.color = col
        when (kind) {
            1 -> corner(c, cx, cy, r, dir, sev, flags)
            2 -> {   // a crest: a hump over a road line
                val p = Path(); p.moveTo(cx - 0.55f * r, cy + 0.3f * r); p.cubicTo(cx - 0.25f * r, cy + 0.3f * r, cx - 0.3f * r, cy - 0.35f * r, cx, cy - 0.35f * r)
                p.cubicTo(cx + 0.3f * r, cy - 0.35f * r, cx + 0.25f * r, cy + 0.3f * r, cx + 0.55f * r, cy + 0.3f * r); c.drawPath(p, line)
                if (flags and 16 != 0) badge(c, cx - 0.62f * r, cy - 0.62f * r, r * 0.32f)
            }
            3 -> {   // a junction: the road goes on, a branch leaves to one side
                c.drawLine(cx, cy + 0.6f * r, cx, cy - 0.55f * r, line)
                c.drawLine(cx, cy + 0.05f * r, cx + dir * 0.55f * r, cy + 0.05f * r, line)
                arrowHead(c, cx + dir * 0.55f * r, cy + 0.05f * r, if (dir < 0) 270f else 90f, r * 0.3f)
            }
            4 -> {   // the finish: a checkered flag
                val s = r * 0.32f
                for (i in 0..1) for (j in 0..1) { solid.color = if ((i + j) % 2 == 0) 0xFFFFFFFF.toInt() else 0xFF202124.toInt(); c.drawRect(cx - s + i * s, cy - s + j * s, cx + i * s, cy + j * s, solid) }
                solid.color = col
            }
        }
    }

    private fun corner(c: Canvas, cx: Float, cy: Float, r: Float, dir: Float, sev: Int, flags: Int) {
        val path = Path()
        if (sev == 0) {   // a hairpin: up one side, round and back down the other
            val rr = 0.22f * r
            path.moveTo(cx - dir * rr, cy + 0.55f * r); path.lineTo(cx - dir * rr, cy - 0.1f * r)
            path.arcTo(RectF(cx - rr, cy - 0.1f * r - rr, cx + rr, cy - 0.1f * r + rr).also { it.offset(0f, 0f) }, if (dir > 0) 180f else 0f, if (dir > 0) 180f else -180f)
            path.lineTo(cx + dir * rr, cy + 0.2f * r)
            c.drawPath(path, line)
            arrowHead(c, cx + dir * rr, cy + 0.2f * r, 180f, r * 0.3f)
        } else {
            val a = Math.toRadians(PaceStyle.bend(sev).toDouble())
            val px = cx; val py = cy + 0.08f * r; val len = 0.52f * r
            val ex = px + dir * (sin(a) * len).toFloat(); val ey = py - (cos(a) * len).toFloat()
            path.moveTo(cx, cy + 0.58f * r); path.lineTo(px, py); path.lineTo(ex, ey)
            c.drawPath(path, line)
            val deg = Math.toDegrees(Math.atan2((ex - px).toDouble(), (py - ey).toDouble())).toFloat()   // 0 = up, 90 = right
            arrowHead(c, ex, ey, deg, r * 0.3f)
        }
        if (flags and 4 != 0) c.drawLine(cx - 0.3f * r, cy + 0.72f * r, cx + 0.3f * r, cy + 0.72f * r, line)   // a long corner: an underline
        label.color = PaceStyle.color(sev); label.textSize = r * 0.62f
        c.drawText(if (sev == 0) "Hp" else sev.toString(), cx + 0.6f * r, cy + 0.82f * r, label)
        if (flags and 1 != 0) badge(c, cx - 0.66f * r, cy - 0.66f * r, r * 0.32f)   // it tightens: caution
    }

    /** An arrowhead at (x, y) pointing in the direction `deg` (0 up, 90 right). */
    private fun arrowHead(c: Canvas, x: Float, y: Float, deg: Float, size: Float) {
        c.save(); c.rotate(deg, x, y)
        val p = Path().apply { moveTo(x, y - size * 0.7f); lineTo(x - size * 0.75f, y + size * 0.5f); lineTo(x + size * 0.75f, y + size * 0.5f); close() }
        c.drawPath(p, solid); c.restore()
    }

    /** A small orange caution disc with an exclamation mark. */
    private fun badge(c: Canvas, x: Float, y: Float, r: Float) {
        solid.color = PaceStyle.ORANGE; c.drawCircle(x, y, r, solid)
        label.color = 0xFF202124.toInt(); label.textSize = r * 1.5f; c.drawText("!", x, y + r * 0.52f, label)
    }
}
