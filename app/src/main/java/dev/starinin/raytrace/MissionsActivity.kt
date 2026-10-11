package dev.starinin.raytrace

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Surface
import io.github.xtratter.uikit.M3Widgets
import java.io.File

/** The current list of four random missions; tap a card for the briefing, then Start. */
class MissionsActivity : Activity() {
    private lateinit var list: LinearLayout
    private lateinit var header: TextView
    private val profile get() = File(filesDir, "profile.json").path
    private var open = -1

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val dp = resources.displayMetrics.density
        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding((20 * dp).toInt(), (16 * dp).toInt(), (20 * dp).toInt(), (12 * dp).toInt()) }
        val top = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        header = TextView(this).apply { textSize = 20f; typeface = M3.medium; setTextColor(M3.TEXT) }
        top.addView(header, LinearLayout.LayoutParams(0, -2, 1f))
        top.addView(M3Widgets.button(this, t("Refresh", "Обновить"), M3Widgets.ButtonKind.TONAL) { open = -1; build(true) }, LinearLayout.LayoutParams((140 * dp).toInt(), (44 * dp).toInt()))
        root.addView(top)
        list = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        root.addView(ScrollView(this).apply { addView(list) }, LinearLayout.LayoutParams(-1, 0, 1f))
        setContentView(root)
    }

    override fun onResume() { super.onResume(); build(false) }

    private fun medal(m: Int) = when (m) { 1 -> "🥉"; 2 -> "🥈"; 3 -> "🥇"; else -> "—" }
    private fun mmss(sec: Float) = "%d:%02d".format(sec.toInt() / 60, sec.toInt() % 60)

    private fun build(refresh: Boolean) {
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val info = Native.missionsInfo(profile, System.nanoTime() and 0xFFFFFFL, refresh)
        header.text = t("Missions", "Миссии") + "  ·  " + info[0].toInt() + " $"
        list.removeAllViews()
        for (i in 0 until 4) {
            val o = 4 + i * 8
            val kind = info[o].toInt(); val diff = info[o + 2].toInt(); val len = info[o + 3]; val limit = info[o + 4]
            val base = info[o + 5]; val best = info[o + 6].toInt(); val bestT = info[o + 7]
            val card = LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL; setPadding(px(16f), px(12f), px(16f), px(12f)); background = M3Surface(this@MissionsActivity, 20f, M3.surface)
            }
            card.addView(TextView(this).apply {
                text = (if (kind == 0) t("Delivery", "Доставка") else t("Race", "Гонка")) + "   " + "★".repeat(diff)
                textSize = 18f; typeface = M3.medium; setTextColor(M3.TEXT)
            })
            card.addView(TextView(this).apply {
                text = "%d m · %s · %d $   %s%s".format(len.toInt(), mmss(limit), base.toInt(), medal(best), if (bestT > 0f) "  " + mmss(bestT) else "")
                textSize = 14f; setTextColor(M3.TEXT)
            })
            val detail = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; visibility = if (open == i) View.VISIBLE else View.GONE }
            detail.addView(TextView(this).apply {
                text = (if (kind == 0) t("Deliver the cargo before time runs out; hits, hard landings and rollovers damage it.", "Довези груз до конца времени; удары, жёсткие посадки и перевороты его портят.")
                        else t("Pass every checkpoint in order before time runs out.", "Проедь все чекпойнты по порядку до конца времени.")) + "\n" +
                    t("Reward: ", "Награда: ") + "🥉 ${base.toInt()}  🥈 ${(base * 1.5f).toInt()}  🥇 ${(base * 2.2f).toInt()}"
                textSize = 14f; setTextColor(M3.TEXT)
            })
            detail.addView(M3Widgets.button(this, t("Start", "Старт"), M3Widgets.ButtonKind.FILLED) {
                startActivity(Intent(this, GameActivity::class.java).putExtra("mission", i).putExtra("profile", profile).putExtra("seed", System.nanoTime() and 0xFFFFFFL))
            }, LinearLayout.LayoutParams(px(160f), px(46f)).apply { topMargin = px(8f) })
            card.addView(detail)
            card.setOnClickListener { open = if (open == i) -1 else i; build(false) }
            list.addView(card, LinearLayout.LayoutParams(-1, -2).apply { topMargin = px(10f) })
        }
    }
}
