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

/** The co-driver voices: listen to each (a sample of calls) and choose one per language, or the system synthesizer. */
class VoiceMenuActivity : Activity() {
    private lateinit var col: LinearLayout
    private lateinit var player: VoicePlayer
    private var tts: android.speech.tts.TextToSpeech? = null

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        player = VoicePlayer(this)
        val dp = resources.displayMetrics.density
        col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL; setPadding((24 * dp).toInt(), (16 * dp).toInt(), (24 * dp).toInt(), (24 * dp).toInt()); setBackgroundColor(M3.base) }
        setContentView(ScrollView(this).apply { isFillViewport = true; setBackgroundColor(M3.base); addView(col, android.widget.FrameLayout.LayoutParams(-1, -2)) })
        build()
    }

    override fun onDestroy() { player.release(); tts?.shutdown(); super.onDestroy() }

    private fun build() {
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        col.removeAllViews()
        col.addView(TextView(this).apply { text = t("Co-driver voice", "Голос штурмана"); textSize = 28f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT) })
        col.addView(TextView(this).apply {
            text = t("▶ plays a sample of calls; Choose sets the voice for that language (the device language is used in rallies).", "▶ проигрывает пример подсказок; «Выбрать» задаёт голос для этого языка (в ралли берётся язык устройства).")
            textSize = 13f; gravity = Gravity.CENTER; setTextColor(M3.TEXT)
        }, LinearLayout.LayoutParams(px(600f), -2).apply { topMargin = px(6f) })
        val chosen = mapOf("ru" to Controls.voice(this, "ru"), "en" to Controls.voice(this, "en"))
        fun row(title: String, sub: String, on: Boolean, play: () -> Unit, choose: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            val label = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
            label.addView(TextView(this).apply { text = (if (on) "✓ " else "") + title; textSize = 16f; setTextColor(M3.TEXT) })
            label.addView(TextView(this).apply { text = sub; textSize = 12f; setTextColor(M3.TEXT) })
            r.addView(label, LinearLayout.LayoutParams(0, -2, 1f))
            r.addView(M3Widgets.button(this, "▶", M3Widgets.ButtonKind.TONAL) { play() }, LinearLayout.LayoutParams(px(64f), px(44f)))
            r.addView(M3Widgets.button(this, t("Choose", "Выбрать"), if (on) M3Widgets.ButtonKind.FILLED else M3Widgets.ButtonKind.TONAL) { choose(); build() }, LinearLayout.LayoutParams(px(130f), px(44f)).apply { leftMargin = px(8f) })
            col.addView(r, LinearLayout.LayoutParams(px(600f), -2).apply { topMargin = px(10f) })
        }
        val sample = listOf("L3", "tightens", "d150", "-", "R2", "into", "HL", "-", "jump", "-", "junc_l", "-", "finish")
        val voices = player.voices()
        for (lang in listOf("ru", "en")) {
            col.addView(TextView(this).apply { text = if (lang == "ru") "Русский" else "English"; textSize = 18f; typeface = M3.medium; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(px(600f), -2).apply { topMargin = px(18f) })
            row(t("System voice", "Системный голос"), t("the speech synthesizer of the device", "синтезатор речи устройства"), chosen[lang] == "system",
                { speakSystem(lang) }, { Controls.setVoice(this, lang, "system") })
            for (v in voices.filter { it.lang == lang }) row(v.name, "${v.license} · ${v.source}", chosen[lang] == v.id, { tts?.stop(); player.play(v.id, sample) }, { Controls.setVoice(this, lang, v.id) })
        }
        col.addView(M3Widgets.button(this, t("Back", "Назад"), M3Widgets.ButtonKind.FILLED) { finish() }, LinearLayout.LayoutParams(px(240f), px(52f)).apply { topMargin = px(22f) })
    }

    private fun speakSystem(lang: String) {
        player.stop()
        val speak = {
            tts?.setLanguage(if (lang == "ru") java.util.Locale("ru") else java.util.Locale.US)
            tts?.speak(if (lang == "ru") "Лево три затягивается, сто пятьдесят. Прыжок." else "Left three tightens, one fifty. Jump.", android.speech.tts.TextToSpeech.QUEUE_FLUSH, null, "sample")
            Unit
        }
        if (tts == null) tts = android.speech.tts.TextToSpeech(this) { if (it == android.speech.tts.TextToSpeech.SUCCESS) speak() } else speak()
    }
}
