package dev.starinin.rallynvx

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.content.res.ColorStateList
import android.view.Gravity
import android.widget.TextView
import io.github.xtratter.uikit.M3Widgets

/**
 * The game's own plain look: flat dark panels with a hairline edge, white text, buttons that are a thin outline,
 * one accent (white). Not the SDK's Material 3 pills and tonal fills.
 */
object Mini {
    const val BG = 0xE0101216.toInt()
    const val INK = 0xFF101216.toInt()
    const val TEXT = 0xFFEDEFF2.toInt()
    const val TEXT2 = 0xB0EDEFF2.toInt()
    const val TEXT3 = 0x70EDEFF2
    const val EDGE = 0x2EFFFFFF
    const val EDGE_STRONG = 0x59FFFFFF
    const val WASH = 0x14FFFFFF
    const val DANGER = 0xFFFF8A80.toInt()

    private fun dp(ctx: Context) = ctx.resources.displayMetrics.density

    fun shape(ctx: Context, fill: Int, edge: Int, radiusDp: Float): GradientDrawable = GradientDrawable().apply {
        setColor(fill); cornerRadius = radiusDp * dp(ctx); if (edge != 0) setStroke(maxOf(1, (dp(ctx) + 0.5f).toInt()), edge)
    }
    private fun ripple(ctx: Context, base: android.graphics.drawable.Drawable, radiusDp: Float) =
        RippleDrawable(ColorStateList.valueOf(0x33FFFFFF), base, shape(ctx, Color.WHITE, 0, radiusDp))

    /** A panel or a pill that sits over the picture. */
    fun panel(ctx: Context, radiusDp: Float = 12f) = shape(ctx, BG, EDGE, radiusDp)
    /** A flat card inside a panel. */
    fun card(ctx: Context, radiusDp: Float = 8f) = shape(ctx, WASH, 0, radiusDp)

    /** A button: FILLED is the one white block, TONAL a faint wash, OUTLINED a hairline, DANGER a red hairline. Same signature as the SDK's, to be a drop-in. */
    fun button(ctx: Context, text: CharSequence, kind: M3Widgets.ButtonKind = M3Widgets.ButtonKind.FILLED, onClick: () -> Unit) = TextView(ctx).apply {
        this.text = text
        textSize = 13.5f; typeface = Typeface.DEFAULT; gravity = Gravity.CENTER; includeFontPadding = false
        val d = dp(ctx); setPadding((14 * d).toInt(), 0, (14 * d).toInt(), 0)
        val base = when (kind) {
            M3Widgets.ButtonKind.FILLED -> { setTextColor(INK); shape(ctx, 0xFFEDEFF2.toInt(), 0, 8f) }
            M3Widgets.ButtonKind.TONAL -> { setTextColor(TEXT); shape(ctx, 0x1FFFFFFF, EDGE, 8f) }
            M3Widgets.ButtonKind.OUTLINED -> { setTextColor(TEXT); shape(ctx, 0, EDGE_STRONG, 8f) }
            M3Widgets.ButtonKind.DANGER -> { setTextColor(DANGER); shape(ctx, 0, 0x80FF8A80.toInt(), 8f) }
        }
        background = ripple(ctx, base, 8f)
        setOnClickListener { onClick() }
    }

    /** A segment of a connected group: the chosen one is the white block. */
    fun segment(ctx: Context, on: Boolean, view: TextView) {
        view.setTextColor(if (on) INK else TEXT)
        view.background = ripple(ctx, if (on) shape(ctx, 0xFFEDEFF2.toInt(), 0, 8f) else shape(ctx, 0, EDGE, 8f), 8f)
    }

    /** A rail item: the chosen one is a faint wash with a white bar on its left. */
    fun railItem(ctx: Context, on: Boolean, view: TextView) {
        view.setTextColor(if (on) TEXT else TEXT2)
        view.background = ripple(ctx, if (on) shape(ctx, 0x1FFFFFFF, 0, 8f) else shape(ctx, 0, 0, 8f), 8f)
    }
}
