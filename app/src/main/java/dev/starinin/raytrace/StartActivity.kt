package dev.starinin.raytrace

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets

/** Launcher screen: free drive (the game) or the scene viewer. */
class StartActivity : Activity() {
    private lateinit var quality: TextView

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val col = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER
            setPadding(px(24f), px(16f), px(24f), px(16f))
        }
        col.addView(TextView(this).apply {
            text = "Raytrace"; textSize = 34f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT)
        })
        fun add(v: android.view.View) = col.addView(v, LinearLayout.LayoutParams(px(280f), px(54f)).apply { topMargin = px(14f) })
        add(M3Widgets.button(this, t("Free drive", "Свободная езда"), M3Widgets.ButtonKind.FILLED) {
            startActivity(Intent(this, GameActivity::class.java).putExtra("seed", System.nanoTime()))
        })
        add(M3Widgets.button(this, t("Scenes", "Сцены"), M3Widgets.ButtonKind.TONAL) {
            startActivity(Intent(this, MainActivity::class.java))
        })
        quality = M3Widgets.button(this, "", M3Widgets.ButtonKind.TONAL) {
            val i = (Quality.index(this) + 1) % Quality.NAMES.size
            Quality.save(this, i); refresh()
        }
        add(quality)
        refresh()
        setContentView(col)
    }

    private fun refresh() { quality.text = t("Quality: ", "Качество: ") + Quality.label(Quality.index(this)) }
}

/** Game resolution scale: Low / Medium / High, saved in the app prefs. */
object Quality {
    val NAMES = arrayOf("Low" to "Низкое", "Medium" to "Среднее", "High" to "Высокое")
    val SCALES = floatArrayOf(0.5f, 0.7f, 1.0f)
    fun label(i: Int) = NAMES[i].let { t(it.first, it.second) }
    fun index(a: Activity) = a.getSharedPreferences("prefs", Activity.MODE_PRIVATE).getInt("game_quality", 1).coerceIn(0, 2)
    fun save(a: Activity, i: Int) { a.getSharedPreferences("prefs", Activity.MODE_PRIVATE).edit().putInt("game_quality", i).apply() }
    fun scale(a: Activity) = SCALES[index(a)]
}
