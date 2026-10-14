package dev.starinin.rallynvx

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets

/** Game settings: rows of a label and a button that cycles the value (no sliders). Saved immediately. */
class SettingsActivity : Activity() {
    private val updaters = ArrayList<() -> Unit>()

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL; setPadding(px(24f), px(16f), px(24f), px(24f)) }
        col.addView(TextView(this).apply { text = t("Settings", "Настройки"); textSize = 28f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT) })

        fun row(label: String, value: () -> String, onClick: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = label; textSize = 16f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val btn = M3Widgets.button(this, value(), M3Widgets.ButtonKind.TONAL) {}
            btn.setOnClickListener { onClick(); (btn as? TextView)?.text = value() }
            updaters.add { (btn as? TextView)?.text = value() }
            r.addView(btn, LinearLayout.LayoutParams(px(190f), px(48f)))
            col.addView(r, LinearLayout.LayoutParams(px(520f), -2).apply { topMargin = px(12f) })
        }
        row(t("Graphics quality", "Качество графики"), { Quality.label(Quality.index(this)) + " ⟳" }) { Quality.save(this, DriveMath.cycle(Quality.index(this), Quality.NAMES.size)) }
        row(t("Render mode", "Режим рендера"), { if (Controls.renderMode(this) == 1) t("Hybrid", "Гибрид") else t("Raster", "Растр") }) { Controls.setRenderMode(this, 1 - Controls.renderMode(this)) }
        row(t("Auto fallback to Raster", "Откат на Растр"), { if (Controls.autoFallback(this)) t("On", "Вкл") else t("Off", "Выкл") }) { Controls.setAutoFallback(this, !Controls.autoFallback(this)) }
        row(t("Dust", "Пыль"), { arrayOf(t("Off", "Выкл"), t("Normal", "Обычная"), t("Lots", "Много"))[Controls.dust(this)] }) { Controls.setDust(this, DriveMath.cycle(Controls.dust(this), 3)) }
        row(t("Autopilot style", "Стиль автопилота"), { apStyleName(Controls.autopilotStyle(this)) }) { Controls.setAutopilotStyle(this, DriveMath.cycle(Controls.autopilotStyle(this), Controls.AP_STYLES)) }
        fun apRow(label: String, key: String, size: Int, show: (Int) -> String) =
            row(label, { show(Controls.apStep(this, key, size)) }) { Controls.setApStep(this, key, DriveMath.cycle(Controls.apStep(this, key, size), size)) }
        // the own style ("Custom"): how much the autopilot risks and how fast it goes
        apRow(t("Own style: cornering speed", "Свой стиль: скорость в поворотах"), "ap_lat", Controls.AP_LAT.size) { "${(Controls.AP_LAT[it] * 100).toInt()} %" }
        apRow(t("Own style: late braking", "Свой стиль: позднее торможение"), "ap_brake", Controls.AP_BRAKE.size) { "${(Controls.AP_BRAKE[it] * 100).toInt()} %" }
        apRow(t("Own style: top speed", "Свой стиль: макс. скорость"), "ap_speed", Controls.AP_SPEED.size) { "${(Controls.AP_SPEED[it] * 100).toInt()} %" }
        apRow(t("Own style: corner cutting", "Свой стиль: срезание поворотов"), "ap_cut", Controls.AP_CUT.size) { "${(Controls.AP_CUT[it] * 100).toInt()} %" }
        apRow(t("Own style: slides", "Свой стиль: заносы"), "ap_slide", 2) { if (it == 1) t("On", "Вкл") else t("Off", "Выкл") }
        apRow(t("Own style: margin to the edge", "Свой стиль: запас до края"), "ap_edge", Controls.AP_EDGE.size) { "${Controls.AP_EDGE[it]} ${t("m", "м")}" }
        row(t("Steering", "Руль"), { arrayOf(t("Buttons", "Кнопки"), t("Wheel", "Колесо"), t("Tilt", "Наклон"))[Controls.mode(this)] }) { Controls.setMode(this, DriveMath.cycle(Controls.mode(this), Controls.MODES)) }
        row(t("Sensitivity", "Чувствительность"), { arrayOf(t("Low", "Низкая"), t("Normal", "Обычная"), t("High", "Высокая"))[Controls.sens(this)] }) { Controls.setSens(this, DriveMath.cycle(Controls.sens(this), 3)) }
        row(t("Volume", "Громкость"), { "${(DriveMath.VOLUME[Controls.volumeStep(this)] * 100).toInt()} %" }) {
            Controls.setVolumeStep(this, DriveMath.cycle(Controls.volumeStep(this), DriveMath.VOLUME.size)); Native.audioVolume(Controls.volume(this))
        }
        row(t("Pedal force", "Усилие педалей"), { if (Controls.pedalsAnalog(this)) t("Adjustable", "Регулируемое") else t("Full on touch", "Полное по нажатию") }) { Controls.setPedalsAnalog(this, !Controls.pedalsAnalog(this)) }
        row(t("Pedals", "Педали"), { if (Controls.pedalsLeft(this)) t("Left", "Слева") else t("Right", "Справа") }) { Controls.setPedalsLeft(this, !Controls.pedalsLeft(this)) }
        var calibrated = false
        row(t("Tilt centre", "Центр наклона"), { if (calibrated) t("Set at start ✓", "Задаётся при старте ✓") else t("Recalibrate", "Перекалибровать") }) { calibrated = true; Controls.setWantsCalibration(this, true) }
        var hintsReset = false
        row(t("Hints", "Подсказки"), { if (hintsReset) t("Will show again ✓", "Покажутся снова ✓") else t("Show again", "Показать снова") }) { hintsReset = true; Controls.resetHints(this) }
        col.addView(M3Widgets.button(this, t("Back", "Назад"), M3Widgets.ButtonKind.FILLED) { finish() }, LinearLayout.LayoutParams(px(240f), px(52f)).apply { topMargin = px(20f) })
        col.addView(TextView(this).apply { text = t("Graphics quality applies at the next game start.", "Качество графики применится при следующем запуске игры."); textSize = 13f; setTextColor(M3.TEXT); gravity = Gravity.CENTER }, LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(10f) })
        col.setBackgroundColor(M3.base)
        setContentView(ScrollView(this).apply { isFillViewport = true; setBackgroundColor(M3.base); addView(col, android.widget.FrameLayout.LayoutParams(-1, -2)) })
    }

    private fun apStyleName(i: Int) = arrayOf(t("Calm", "Спокойный"), t("Normal", "Обычный"), t("Aggressive", "Агрессивный"), t("Own", "Свой"))[i.coerceIn(0, 3)]

    override fun onResume() { super.onResume(); updaters.forEach { it() } }
}
