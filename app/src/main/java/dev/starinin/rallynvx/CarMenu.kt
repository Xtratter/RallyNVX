package dev.starinin.rallynvx

import android.content.Context
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.Switch
import android.widget.TextView
import io.github.xtratter.uikit.Expressive
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets

/**
 * The car's menu in two panes: a rail of sections on the left (overview, aids, engine, transmission, brakes and tyres,
 * suspension), the chosen section on the right. Switches for the aids, connected button groups for the choices
 * (the box, the automatic, the presets), steppers for the powers of the parts. [live] feeds the overview while it is open.
 */
class CarMenu(private val ctx: Context, private val en: Boolean, private val running: () -> Boolean, private val sendGearCfg: () -> Unit, private val close: () -> Unit) {
    private val dp = ctx.resources.displayMetrics.density
    private fun px(v: Float) = (v * dp).toInt()
    private fun t(e: String, r: String) = if (en) e else r
    private val refreshers = ArrayList<() -> Unit>()
    private val rail = LinearLayout(ctx).apply { orientation = LinearLayout.VERTICAL }
    private val body = android.widget.FrameLayout(ctx)
    private val railItems = ArrayList<TextView>()
    private val sections = ArrayList<View>()
    private var shown = 0
    private val cards = Array(4) { TextView(ctx) }

    val view: View = build()

    /** Re-reads every control from the preferences (after the panel was reopened or the tuning was reset). */
    fun refresh() { refreshers.forEach { it() } }

    /** The overview's cards from the stat slots (tyres 88..95, coolant and clutch 96..97, damage 98..103, fuel 104, stability control 105). */
    fun live(s: FloatArray) {
        if (s.size < 106 || shown != 0) return
        cards[0].text = CarMath.tyreLine(floatArrayOf(s[88], s[89], s[90], s[91]), floatArrayOf(s[92], s[93], s[94], s[95]), en)
        cards[1].text = CarMath.heatLine(s[96], s[97], en)
        cards[2].text = CarMath.damageLine(s[98], s[99], floatArrayOf(s[100], s[101], s[102], s[103]), en)
        cards[3].text = CarMath.fuelLine(s[104], en) + (if (s[105] > 0.5f) "   ESC" else "")
    }

    private fun build(): View {
        val head = LinearLayout(ctx).apply { gravity = Gravity.CENTER_VERTICAL }
        head.addView(TextView(ctx).apply { text = t("Car", "Машина"); textSize = 20f; setTextColor(M3.TEXT); typeface = M3.medium }, LinearLayout.LayoutParams(0, -2, 1f))
        head.addView(M3Widgets.button(ctx, t("Close", "Закрыть"), M3Widgets.ButtonKind.FILLED) { close() }, LinearLayout.LayoutParams(px(120f), px(40f)))

        val names = listOf(t("Overview", "Обзор"), t("Aids", "Помощники"), t("Engine", "Двигатель"), t("Transmission", "Трансмиссия"), t("Brakes, tyres", "Тормоза и шины"), t("Suspension", "Подвеска"))
        sections.add(overview()); sections.add(aids()); sections.add(engine()); sections.add(transmission()); sections.add(brakesTyres()); sections.add(suspension())
        for ((i, n) in names.withIndex()) {
            val item = TextView(ctx).apply {
                text = n; textSize = 14f; typeface = M3.medium; gravity = Gravity.CENTER_VERTICAL
                setPadding(px(14f), 0, px(10f), 0); foreground = M3.ripple(ctx, 100f)
                setOnClickListener { select(i) }
            }
            railItems.add(item)
            rail.addView(item, LinearLayout.LayoutParams(-1, px(40f)).apply { bottomMargin = px(2f) })
            val sv = ScrollView(ctx).apply { addView(sections[i]); isVerticalScrollBarEnabled = false; visibility = View.GONE }
            sections[i] = sv
            body.addView(sv, android.widget.FrameLayout.LayoutParams(-1, -1))
        }
        select(0)

        val main = LinearLayout(ctx).apply { orientation = LinearLayout.HORIZONTAL }
        main.addView(ScrollView(ctx).apply { addView(rail); isVerticalScrollBarEnabled = false }, LinearLayout.LayoutParams(px(150f), -1))
        main.addView(View(ctx).apply { setBackgroundColor(M3.ink(0x1A)) }, LinearLayout.LayoutParams(px(1f), -1).apply { marginStart = px(8f); marginEnd = px(14f) })
        main.addView(body, LinearLayout.LayoutParams(px(500f), -1))

        return LinearLayout(ctx).apply {
            orientation = LinearLayout.VERTICAL
            addView(head, LinearLayout.LayoutParams(-1, px(44f)))
            addView(main, LinearLayout.LayoutParams(-2, px(270f)).apply { topMargin = px(6f) })
        }
    }

    private fun select(i: Int) {
        shown = i
        for ((k, item) in railItems.withIndex()) {
            val on = k == i
            item.setTextColor(if (on) M3.onSecondaryContainer else M3.TEXT)
            item.background = if (on) Expressive.pill(ctx, M3.secondaryContainer, 100f) else null
            sections[k].visibility = if (on) View.VISIBLE else View.GONE
        }
    }

    private fun column(title: String): LinearLayout = LinearLayout(ctx).apply {
        orientation = LinearLayout.VERTICAL
        addView(TextView(ctx).apply { text = title; textSize = 15f; setTextColor(M3.primary); typeface = M3.medium }, LinearLayout.LayoutParams(-2, -2).apply { bottomMargin = px(6f) })
    }
    private fun LinearLayout.row(v: View, h: Float = -2f) = addView(v, LinearLayout.LayoutParams(-1, if (h < 0) -2 else px(h)).apply { topMargin = px(4f) })

    // ---- controls -------------------------------------------------------------------------------------------------

    /** A switch row for an aid bit. */
    private fun aidRow(bit: Int): View {
        val r = LinearLayout(ctx).apply { gravity = Gravity.CENTER_VERTICAL; minimumHeight = px(44f); background = M3.ripple(ctx, 12f) }
        r.addView(TextView(ctx).apply { text = CarMath.aidName(bit, en); textSize = 15f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
        val sw = Switch(ctx).apply { isClickable = false; M3Widgets.tint(this) }
        fun load() { sw.isChecked = CarMath.aidOn(Controls.assists(ctx), bit) }
        load(); refreshers.add { load() }
        r.addView(sw)
        r.setOnClickListener {
            val m = CarMath.toggle(Controls.assists(ctx), bit); Controls.setAssists(ctx, m); if (running()) Native.gameAssists(m); load()
        }
        return r
    }

    /** A title over a connected group of [n] buttons: the chosen one is filled, a tap chooses. */
    private fun groupRow(title: String, n: Int, name: (Int) -> String, get: () -> Int, set: (Int) -> Unit): View {
        val col = LinearLayout(ctx).apply { orientation = LinearLayout.VERTICAL }
        col.addView(TextView(ctx).apply { text = title; textSize = 14f; setTextColor(M3.TEXT2) })
        val g = LinearLayout(ctx).apply { orientation = LinearLayout.HORIZONTAL }
        val cells = ArrayList<TextView>()
        for (k in 0 until n) {
            val c = TextView(ctx).apply { text = name(k); textSize = 14f; typeface = M3.medium; gravity = Gravity.CENTER; maxLines = 1; foreground = M3.ripple(ctx, 100f) }
            c.setOnClickListener { set(k); refresh() }
            cells.add(c); g.addView(c, LinearLayout.LayoutParams(0, px(40f), 1f).apply { if (k > 0) marginStart = px(3f) })
        }
        fun load() { val cur = get(); for ((k, c) in cells.withIndex()) {
            val on = k == cur
            c.setTextColor(if (on) M3.onSecondaryContainer else M3.TEXT)
            c.background = if (on) Expressive.pill(ctx, M3.secondaryContainer, 100f) else M3.pill(ctx, M3.surfaceContainer, M3.ink(0x26), 12f)
        } }
        load(); refreshers.add { load() }
        col.addView(g, LinearLayout.LayoutParams(-1, -2).apply { topMargin = px(4f) })
        return col
    }

    /** A part's power: title, minus, the value, plus. */
    private fun stepRow(i: Int): View {
        val r = LinearLayout(ctx).apply { gravity = Gravity.CENTER_VERTICAL }
        r.addView(TextView(ctx).apply { text = CarMath.partName(i, en); textSize = 15f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
        val v = TextView(ctx).apply { textSize = 15f; setTextColor(M3.TEXT); gravity = Gravity.CENTER }
        fun show() { v.text = CarMath.valueText(i, Controls.tuning(ctx, i)) }
        fun change(dir: Int) { val nv = CarMath.step(i, Controls.tuning(ctx, i), dir); Controls.setTuning(ctx, i, nv); if (running()) Native.gameTuning(i, nv); show() }
        show(); refreshers.add { show() }
        r.addView(M3Widgets.button(ctx, "−", M3Widgets.ButtonKind.TONAL) { change(-1) }, LinearLayout.LayoutParams(px(46f), px(38f)))
        r.addView(v, LinearLayout.LayoutParams(px(64f), -2))
        r.addView(M3Widgets.button(ctx, "+", M3Widgets.ButtonKind.TONAL) { change(1) }, LinearLayout.LayoutParams(px(46f), px(38f)))
        return r
    }

    private fun LinearLayout.steps(vararg parts: Int) { for (p in parts) row(stepRow(p), 42f) }

    // ---- sections -------------------------------------------------------------------------------------------------

    private fun overview(): View {
        val c = column(t("State of the car", "Состояние машины"))
        val titles = listOf(t("Tyres", "Шины"), t("Heat", "Нагрев"), t("Damage", "Повреждения"), t("Fuel", "Топливо"))
        fun card(k: Int) = LinearLayout(ctx).apply {
            orientation = LinearLayout.VERTICAL; setPadding(px(12f), px(8f), px(12f), px(8f)); background = M3.pill(ctx, M3.surfaceContainer, M3.ink(0x26), 12f)
            addView(TextView(ctx).apply { text = titles[k]; textSize = 12f; setTextColor(M3.TEXT2) })
            cards[k].apply { textSize = 13f; setTextColor(M3.TEXT); text = "—" }
            addView(cards[k])
        }
        for (rowIdx in 0..1) {
            val r = LinearLayout(ctx)
            r.addView(card(rowIdx * 2), LinearLayout.LayoutParams(0, -1, 1f))
            r.addView(card(rowIdx * 2 + 1), LinearLayout.LayoutParams(0, -1, 1f).apply { marginStart = px(8f) })
            c.addView(r, LinearLayout.LayoutParams(-1, -2).apply { topMargin = px(6f) })
        }
        val b = LinearLayout(ctx)
        b.addView(M3Widgets.button(ctx, t("Service", "Сервис"), M3Widgets.ButtonKind.TONAL) { if (running()) Native.gameService() }, LinearLayout.LayoutParams(0, px(42f), 1f))
        b.addView(M3Widgets.button(ctx, t("Reset tuning", "Сбросить настройки"), M3Widgets.ButtonKind.OUTLINED) { resetAll() }, LinearLayout.LayoutParams(0, px(42f), 1.3f).apply { marginStart = px(8f) })
        c.addView(b, LinearLayout.LayoutParams(-1, -2).apply { topMargin = px(12f) })
        c.addView(TextView(ctx).apply { text = t("Service repairs the car, fits new tyres and fills the tank.", "Сервис чинит машину, ставит новые шины и заправляет бак."); textSize = 12f; setTextColor(M3.TEXT3) }, LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(6f) })
        return c
    }

    private fun resetAll() {
        for (i in 0 until CarMath.PARTS) { Controls.setTuning(ctx, i, 1f); if (running()) Native.gameTuning(i, 1f) }
        Controls.setDiffPreset(ctx, 1); Controls.setChassisPreset(ctx, 0)
        Controls.setAssists(ctx, CarMath.DEFAULT_AIDS); if (running()) Native.gameAssists(CarMath.DEFAULT_AIDS); refresh()
    }

    private fun aids(): View {
        val c = column(t("Driver aids", "Помощники водителя"))
        for (bit in 0 until CarMath.AIDS) c.row(aidRow(bit), 44f)
        return c
    }

    private fun engine(): View {
        val c = column(t("Engine", "Двигатель"))
        c.steps(0, 1, 6)
        return c
    }

    private fun transmission(): View {
        val c = column(t("Transmission", "Трансмиссия"))
        c.row(groupRow(t("Manual box", "Ручная коробка"), GearMath.BOXES, { GearMath.boxName(it, en) }, { Controls.boxType(ctx) }) { Controls.setBoxType(ctx, it); sendGearCfg() })
        c.row(groupRow(t("Automatic", "Автомат"), GearMath.PROFILES, { GearMath.profileName(it, en) }, { Controls.shiftProfile(ctx) }) { Controls.setShiftProfile(ctx, it); sendGearCfg() })
        c.row(groupRow(t("Differentials", "Дифференциалы"), CarMath.PRESET_COUNT, { CarMath.presetName(it, en) }, { Controls.diffPreset(ctx) }) { n ->
            Controls.setDiffPreset(ctx, n)
            for (k in 0 until 4) { Controls.setTuning(ctx, 7 + k, CarMath.PRESETS[n][k]); if (running()) Native.gameTuning(7 + k, CarMath.PRESETS[n][k]) }
        })
        c.steps(7, 8, 9, 10)
        return c
    }

    private fun brakesTyres(): View {
        val c = column(t("Brakes and tyres", "Тормоза и шины"))
        c.steps(2, 11, 3, 12)
        return c
    }

    private fun suspension(): View {
        val c = column(t("Suspension and aero", "Подвеска и аэродинамика"))
        c.row(groupRow(t("Preset", "Пресет"), CarMath.CHASSIS_PRESET_COUNT, { CarMath.chassisPresetName(it, en) }, { Controls.chassisPreset(ctx) }) { n ->
            Controls.setChassisPreset(ctx, n)
            for (k in 0 until 6) { Controls.setTuning(ctx, 13 + k, CarMath.CHASSIS_PRESETS[n][k]); if (running()) Native.gameTuning(13 + k, CarMath.CHASSIS_PRESETS[n][k]) }
        })
        c.steps(4, 13, 14, 15, 16, 17, 18, 5)
        return c
    }
}
