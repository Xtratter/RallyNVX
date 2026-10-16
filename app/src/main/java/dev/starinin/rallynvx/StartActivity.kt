package dev.starinin.rallynvx

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets
import java.io.File

/** Main menu: free drive, scene viewer, garage, settings. */
class StartActivity : Activity() {
    private lateinit var money: TextView
    private val profile get() = File(filesDir, "profile.json").path

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
            text = "RallyNVX"; textSize = 34f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT)
        })
        money = TextView(this).apply { textSize = 16f; gravity = Gravity.CENTER; setTextColor(M3.TEXT) }
        col.addView(money)
        fun add(v: android.view.View) = col.addView(v, LinearLayout.LayoutParams(px(280f), px(54f)).apply { topMargin = px(14f) })
        add(M3Widgets.button(this, t("Free drive", "Свободная езда"), M3Widgets.ButtonKind.FILLED) {
            startActivity(Intent(this, GameActivity::class.java).putExtra("seed", System.nanoTime() and 0xFFFFFFL).putExtra("profile", profile))
        })
        add(M3Widgets.button(this, t("Test track", "Испытательный полигон"), M3Widgets.ButtonKind.TONAL) {
            startActivity(Intent(this, GameActivity::class.java).putExtra("seed", 1L).putExtra("profile", profile).putExtra("testtrack", true))
        })
        add(M3Widgets.button(this, t("Rally", "Ралли"), M3Widgets.ButtonKind.FILLED) {
            startActivity(Intent(this, RallyMenuActivity::class.java).putExtra("profile", profile))
        })
        add(M3Widgets.button(this, t("Scenes", "Сцены"), M3Widgets.ButtonKind.TONAL) {
            startActivity(Intent(this, MainActivity::class.java))
        })
        add(M3Widgets.button(this, t("Garage", "Гараж"), M3Widgets.ButtonKind.TONAL) {
            startActivity(Intent(this, GarageActivity::class.java))
        })
        add(M3Widgets.button(this, t("Settings", "Настройки"), M3Widgets.ButtonKind.TONAL) {
            startActivity(Intent(this, SettingsActivity::class.java))
        })
        col.setBackgroundColor(M3.base)
        setContentView(android.widget.ScrollView(this).apply { isFillViewport = true; setBackgroundColor(M3.base); addView(col, android.widget.FrameLayout.LayoutParams(-1, -1)) })
        refresh()
    }

    override fun onResume() { super.onResume(); refresh() }

    private fun refresh() {
        money.visibility = android.view.View.GONE   // nothing earns money until the rally career (the garage is free for now)
    }
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
