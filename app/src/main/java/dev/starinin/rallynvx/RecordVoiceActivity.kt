package dev.starinin.rallynvx

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.Gravity
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets
import org.json.JSONObject

/** Records the co-driver's phrases with the player's own voice, one clip at a time (extra "lang": ru or en). */
class RecordVoiceActivity : Activity() {
    private lateinit var lang: String
    private lateinit var list: LinearLayout
    private lateinit var progress: TextView
    private lateinit var player: VoicePlayer
    private val recorder = ClipRecorder()
    private var recording: String? = null
    private var pendingClip: String? = null
    private var texts: List<Pair<String, String>> = emptyList()   // clip id -> the words
    private val rows = HashMap<String, Pair<TextView, TextView>>()   // clip id -> (status mark, record button)

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        lang = intent.getStringExtra("lang") ?: "ru"
        player = VoicePlayer(this)
        val json = JSONObject(assets.open("voices/clips.json").bufferedReader().use { it.readText() }).getJSONObject(lang)
        texts = json.keys().asSequence().map { it to json.getString(it) }.toList()
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL; setPadding(px(24f), px(16f), px(24f), px(24f)); setBackgroundColor(M3.base) }
        col.addView(TextView(this).apply { text = t("My voice", "Мой голос") + " — " + (if (lang == "ru") "русский" else "English"); textSize = 26f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT) })
        col.addView(TextView(this).apply {
            text = t("Tap Record, say the phrase, tap Stop. Silence is trimmed and the loudness matched. A call uses your voice only when all its phrases are recorded, otherwise the system voice speaks it.",
                "Нажмите «Запись», скажите фразу, нажмите «Стоп». Тишина обрезается, громкость выравнивается. Подсказка звучит вашим голосом, только если записаны все её фразы, иначе читает системный голос.")
            textSize = 13f; gravity = Gravity.CENTER; setTextColor(M3.TEXT)
        }, LinearLayout.LayoutParams(px(620f), -2).apply { topMargin = px(6f) })
        progress = TextView(this).apply { textSize = 15f; gravity = Gravity.CENTER; setTextColor(M3.TEXT) }
        col.addView(progress, LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(8f) })
        list = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        col.addView(list)
        for ((id, text) in texts) addRow(id, text, px(1f).toFloat(), dp)
        col.addView(M3Widgets.button(this, t("Done", "Готово"), M3Widgets.ButtonKind.FILLED) { finish() }, LinearLayout.LayoutParams(px(240f), px(52f)).apply { topMargin = px(22f) })
        setContentView(ScrollView(this).apply { isFillViewport = true; setBackgroundColor(M3.base); addView(col, android.widget.FrameLayout.LayoutParams(-1, -2)) })
        refresh()
    }

    private fun addRow(id: String, text: String, unit: Float, dp: Float) {
        fun px(v: Float) = (v * dp).toInt()
        val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        val mark = TextView(this).apply { textSize = 18f; setTextColor(M3.TEXT); gravity = Gravity.CENTER }
        r.addView(mark, LinearLayout.LayoutParams(px(32f), -2))
        r.addView(TextView(this).apply { this.text = text.replaceFirstChar { it.uppercase() }; textSize = 17f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
        val recBtn = M3Widgets.button(this, t("Record", "Запись"), M3Widgets.ButtonKind.TONAL) { toggle(id) } as TextView
        r.addView(recBtn, LinearLayout.LayoutParams(px(130f), px(44f)))
        r.addView(M3Widgets.button(this, "▶", M3Widgets.ButtonKind.TONAL) { player.play(VoicePlayer.OWN, listOf(id), lang = lang) }, LinearLayout.LayoutParams(px(64f), px(44f)).apply { leftMargin = px(8f) })
        rows[id] = mark to recBtn
        list.addView(r, LinearLayout.LayoutParams(px(620f), -2).apply { topMargin = px(8f) })
    }

    private fun refresh() {
        for ((id, p) in rows) { p.first.text = if (OwnVoice.has(this, lang, id)) "✓" else "○"; p.second.text = if (recording == id) t("Stop", "Стоп") else if (OwnVoice.has(this, lang, id)) t("Again", "Ещё раз") else t("Record", "Запись") }
        progress.text = t("Recorded ", "Записано ") + "${OwnVoice.count(this, lang, texts.map { it.first })} / ${texts.size}"
    }

    private fun toggle(id: String) {
        if (recording == id) {   // stop: clean and save
            val pcm = AudioTrim.clean(recorder.stop())
            recording = null
            if (pcm.size > 1600) OwnVoice.write(this, lang, id, pcm) else android.widget.Toast.makeText(this, t("Nothing heard, try again", "Ничего не слышно, повторите"), android.widget.Toast.LENGTH_SHORT).show()
            refresh(); return
        }
        if (recording != null) { recorder.stop(); recording = null }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) { pendingClip = id; requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 7); return }
        player.stop()
        if (recorder.start()) recording = id else android.widget.Toast.makeText(this, t("The microphone is not available", "Микрофон недоступен"), android.widget.Toast.LENGTH_SHORT).show()
        refresh()
    }

    override fun onRequestPermissionsResult(code: Int, perms: Array<out String>, res: IntArray) {
        super.onRequestPermissionsResult(code, perms, res)
        val id = pendingClip; pendingClip = null
        if (code == 7 && res.isNotEmpty() && res[0] == PackageManager.PERMISSION_GRANTED && id != null) toggle(id)
        else android.widget.Toast.makeText(this, t("Without the microphone permission nothing can be recorded", "Без разрешения на микрофон записать нельзя"), android.widget.Toast.LENGTH_LONG).show()
    }

    override fun onDestroy() { if (recording != null) recorder.stop(); player.release(); super.onDestroy() }
}
