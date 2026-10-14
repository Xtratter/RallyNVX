package dev.starinin.raytrace

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Surface
import io.github.xtratter.uikit.M3Widgets
import java.io.File

/**
 * The garage: the engine shows the chosen car on a turntable (garage scene); a panel on the right buys,
 * selects and upgrades cars. Every action goes through the profile file and the screen is rebuilt from `garageInfo`.
 */
class GarageActivity : Activity(), SurfaceHolder.Callback {
    // debug/test: another profile file in the app files ("profile_file" extra) keeps test purchases away from the real one
    private val profile get() = File(filesDir, intent.getStringExtra("profile_file") ?: "profile.json").path
    private lateinit var root: FrameLayout
    private lateinit var panel: LinearLayout
    private lateinit var money: TextView
    private var shown = -1          // the car on the turntable (index in the vehicles file)
    private var running = false

    private fun carName(i: Int) = when (i) { 0 -> t("Off-roader", "Внедорожник"); 1 -> "Vulpes WRC"; else -> t("Car", "Машина") + " ${i + 1}" }
    private val upgradeNames get() = arrayOf(t("Engine", "Двигатель"), t("Tyres", "Шины"), t("Suspension", "Подвеска"), t("Brakes", "Тормоза"))

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        root = FrameLayout(this)
        val sv = SurfaceView(this); sv.holder.addCallback(this)
        root.addView(sv, FrameLayout.LayoutParams(-1, -1))
        panel = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL
            setPadding(px(16f), px(12f), px(16f), px(12f)); background = M3Surface(this@GarageActivity, 20f, M3.surface)
        }
        root.addView(ScrollView(this).apply { addView(panel) }, FrameLayout.LayoutParams(px(360f), -2, Gravity.END or Gravity.CENTER_VERTICAL).apply { setMargins(0, px(16f), px(16f), px(16f)) })
        money = TextView(this).apply { textSize = 18f; typeface = M3.medium; setTextColor(M3.TEXT); background = M3Surface(this@GarageActivity, 16f, M3.surface); setPadding(px(14f), px(8f), px(14f), px(8f)) }
        root.addView(money, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(16f), 0, 0) })
        root.addView(M3Widgets.button(this, t("Back", "Назад"), M3Widgets.ButtonKind.TONAL) { finish() },
            FrameLayout.LayoutParams(px(120f), px(48f), Gravity.BOTTOM or Gravity.START).apply { setMargins(px(16f), 0, 0, px(16f)) })
        setContentView(root)
    }

    private fun info(): GarageMath.Info? = GarageMath.parse(Native.garageInfo(profile))

    /** Rebuilds the panel from the profile file. */
    private fun refresh() {
        val inf = info() ?: return
        if (shown < 0 || shown >= inf.cars.size) shown = intent.getIntExtra("car", -1).let { if (it in inf.cars.indices) it else inf.selected }   // "car": debug extra to open on a given car
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        money.text = t("Money: ", "Деньги: ") + GarageMath.price(inf.money)
        money.visibility = if (inf.cars.all { it.price == 0 && it.upgradeCost == 0 }) View.GONE else View.VISIBLE   // everything is free for now
        panel.removeAllViews()
        val car = inf.cars[shown]
        panel.addView(TextView(this).apply { text = carName(shown); textSize = 22f; typeface = M3.medium; gravity = Gravity.CENTER; setTextColor(M3.TEXT) })
        val nav = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER }
        nav.addView(M3Widgets.button(this, "◀", M3Widgets.ButtonKind.TONAL) { showCar(GarageMath.step(shown, -1, inf.cars.size)) }, LinearLayout.LayoutParams(px(80f), px(44f)))
        nav.addView(TextView(this).apply { text = "${shown + 1} / ${inf.cars.size}"; textSize = 15f; setTextColor(M3.TEXT); gravity = Gravity.CENTER }, LinearLayout.LayoutParams(px(90f), -2))
        nav.addView(M3Widgets.button(this, "▶", M3Widgets.ButtonKind.TONAL) { showCar(GarageMath.step(shown, 1, inf.cars.size)) }, LinearLayout.LayoutParams(px(80f), px(44f)))
        panel.addView(nav, LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(8f) })
        // buy / select
        val action = when {
            !car.owned -> M3Widgets.button(this, if (car.price == 0) t("Get", "Получить") else t("Buy", "Купить") + " — " + GarageMath.price(car.price), M3Widgets.ButtonKind.FILLED) {
                if (!Native.garageBuyCar(profile, shown)) toast(t("Not enough money", "Не хватает денег")); refresh()
            }.also { if (inf.money < car.price) it.alpha = 0.5f }
            shown == inf.selected -> M3Widgets.button(this, t("Selected ✓", "Выбрана ✓"), M3Widgets.ButtonKind.TONAL) {}.also { it.alpha = 0.7f }
            else -> M3Widgets.button(this, t("Select", "Выбрать"), M3Widgets.ButtonKind.FILLED) { Native.garageSelect(profile, shown); refresh() }
        }
        panel.addView(action, LinearLayout.LayoutParams(px(300f), px(48f)).apply { topMargin = px(10f) })
        if (shown == 1) panel.addView(TextView(this).apply {
            text = t("Model: Peugeot 206 WRC by vecarz (Sketchfab), CC BY-NC-SA 4.0, simplified", "Модель: Peugeot 206 WRC от vecarz (Sketchfab), CC BY-NC-SA 4.0, упрощена")
            textSize = 11f; setTextColor(M3.TEXT); alpha = 0.7f; gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(px(300f), -2).apply { topMargin = px(6f) })
        // upgrades
        for (u in 0 until 4) {
            val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            val level = car.levels[u]
            row.addView(TextView(this).apply { text = upgradeNames[u] + "\n" + GarageMath.pips(level); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val price = GarageMath.upgradePrice(car.upgradeCost, level)
            val label = if (!car.owned) "—" else price?.let { if (it == 0) t("Free", "Бесплатно") else GarageMath.price(it) } ?: t("MAX", "МАКС")
            val btn = M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL) {
                if (car.owned && price != null) { if (!Native.garageBuyUpgrade(profile, shown, u)) toast(t("Not enough money", "Не хватает денег")); refresh() }
            }
            if (!car.owned || price == null || inf.money < price) btn.alpha = 0.5f
            row.addView(btn, LinearLayout.LayoutParams(px(120f), px(44f)))
            panel.addView(row, LinearLayout.LayoutParams(-1, -2).apply { topMargin = px(8f) })
        }
    }

    private fun toast(s: String) = android.widget.Toast.makeText(this, s, android.widget.Toast.LENGTH_SHORT).show()

    private fun showCar(i: Int) { shown = i; if (running) Native.gameGarageCar(i); refresh() }

    override fun surfaceCreated(h: SurfaceHolder) { running = false }
    override fun surfaceChanged(h: SurfaceHolder, f: Int, w: Int, hh: Int) {
        if (running) { Native.gameResize(w, hh); return }
        Native.gameStart(h.surface, w, hh, 7L, profile, true)      // the garage scene with the selected car
        Native.gameQuality(Quality.index(this))
        Native.gameRenderMode(Controls.renderMode(this))
        running = true
        if (shown >= 0) Native.gameGarageCar(shown)
        refresh()
    }
    override fun surfaceDestroyed(h: SurfaceHolder) { Native.gameStop(); running = false }

    override fun onResume() { super.onResume(); refresh() }

    override fun onWindowFocusChanged(f: Boolean) {
        super.onWindowFocusChanged(f)
        if (!f) return
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false)
            window.insetsController?.hide(android.view.WindowInsets.Type.systemBars())
            window.insetsController?.systemBarsBehavior = android.view.WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        } else {
            @Suppress("DEPRECATION")
            window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
        }
    }
}
