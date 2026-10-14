package dev.starinin.rallynvx

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets

/** The list of rallies with the best times; each starts its two stages in the game. */
class RallyMenuActivity : Activity() {
    private lateinit var col: LinearLayout
    private var profile = ""

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        profile = intent.getStringExtra("profile") ?: ""
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL; setPadding(px(24f), px(16f), px(24f), px(24f)); setBackgroundColor(M3.base) }
        setContentView(ScrollView(this).apply { isFillViewport = true; setBackgroundColor(M3.base); addView(col, android.widget.FrameLayout.LayoutParams(-1, -2)) })
    }

    override fun onResume() { super.onResume(); build() }

    private fun build() {
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        col.removeAllViews()
        col.addView(TextView(this).apply { text = t("Rally", "Ралли"); textSize = 28f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT) })
        col.addView(TextView(this).apply {
            text = t("Two stages against the clock: the special stage and the city super-special (2 laps). A reset costs 10 s, leaving the road 5 s, a skipped check line 30 s.",
                "Два этапа на время: спецучасток и городская суперспециалка (2 круга). Сброс стоит 10 с, выезд с дороги 5 с, пропущенная контрольная линия 30 с.")
            textSize = 13f; gravity = Gravity.CENTER; setTextColor(M3.TEXT)
        }, LinearLayout.LayoutParams(px(560f), -2).apply { topMargin = px(8f) })
        for (i in RallyData.SEEDS.indices) {
            val best = RallyData.record(this, i, -1)
            val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            val label = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
            label.addView(TextView(this).apply { text = "${i + 1}. ${RallyData.name(i)}"; textSize = 17f; setTextColor(M3.TEXT) })
            label.addView(TextView(this).apply { text = t("Best: ", "Рекорд: ") + (if (best > 0f) RallyData.time(best) else "—"); textSize = 13f; setTextColor(M3.TEXT) })
            row.addView(label, LinearLayout.LayoutParams(0, -2, 1f))
            row.addView(M3Widgets.button(this, t("Start", "Старт"), M3Widgets.ButtonKind.FILLED) {
                startActivity(Intent(this, GameActivity::class.java).putExtra("seed", RallyData.SEEDS[i]).putExtra("profile", profile).putExtra("rally", i + 1))
            }, LinearLayout.LayoutParams(px(150f), px(48f)))
            col.addView(row, LinearLayout.LayoutParams(px(520f), -2).apply { topMargin = px(14f) })
        }
        col.addView(M3Widgets.button(this, t("Back", "Назад"), M3Widgets.ButtonKind.TONAL) { finish() }, LinearLayout.LayoutParams(px(240f), px(52f)).apply { topMargin = px(22f) })
    }
}
