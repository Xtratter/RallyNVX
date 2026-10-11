package dev.starinin.raytrace

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.*
import android.widget.FrameLayout
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Widgets

/** Free drive: a seeded level, left stick = steer/throttle, right stick down = brake. */
class GameActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var sticks: SticksLayer
    private lateinit var hud: Hud
    private val handler = Handler(Looper.getMainLooper())
    private var running = false
    private var handbrake = false
    private var lx = 0f; private var ly = 0f; private var ry = 0f
    private var seed = 0L
    private val tick = object : Runnable {
        override fun run() {
            val s = Native.gameStats()
            hud.text = "${s[1].toInt()} km/h · ${s[0].toInt()} fps · seed ${s[2].toInt()}"
            handler.postDelayed(this, 250)
        }
    }

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        seed = intent.getLongExtra("seed", System.nanoTime())
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val root = FrameLayout(this)
        val sv = SurfaceView(this)
        sv.holder.addCallback(this)
        val scale = Quality.scale(this)
        if (scale < 0.999f) {
            val dm = resources.displayMetrics
            sv.holder.setFixedSize((dm.widthPixels * scale).toInt().coerceAtLeast(2), (dm.heightPixels * scale).toInt().coerceAtLeast(2))
        }
        root.addView(sv, FrameLayout.LayoutParams(-1, -1))
        sticks = SticksLayer(this).also { it.onChange = { x, y, _, rY -> lx = x; ly = y; ry = rY; sendInput() } }
        root.addView(sticks, FrameLayout.LayoutParams(-1, -1))
        hud = Hud(this)
        root.addView(hud, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(24f), 0, 0) })
        fun button(label: String, gravity: Int, bottom: Float, end: Float, onClick: () -> Unit) =
            M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL, onClick).also {
                root.addView(it, FrameLayout.LayoutParams(px(120f), px(48f), gravity).apply { setMargins(0, px(24f), px(end), px(bottom)) })
            }
        button(t("Reset", "Сброс"), Gravity.TOP or Gravity.END, 0f, 16f) { Native.gameReset() }
        button(t("New level", "Новый уровень"), Gravity.TOP or Gravity.END, 0f, 150f) { Native.gameNewLevel(System.nanoTime()) }
        button(t("Handbrake", "Ручник"), Gravity.BOTTOM or Gravity.END, 220f, 40f) {}.setOnTouchListener { _, e ->
            when (e.actionMasked) {
                MotionEvent.ACTION_DOWN -> { handbrake = true; sendInput() }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> { handbrake = false; sendInput() }
            }
            true
        }
        setContentView(root)
    }

    private fun sendInput() { if (running) Native.gameInput(ly, lx, ry < -0.5f, handbrake) }

    override fun surfaceCreated(h: SurfaceHolder) { running = false }
    override fun surfaceChanged(h: SurfaceHolder, f: Int, w: Int, hh: Int) {
        if (running) { Native.gameResize(w, hh); return }
        Native.gameStart(h.surface, w, hh, seed)
        running = true
    }
    override fun surfaceDestroyed(h: SurfaceHolder) { Native.gameStop(); running = false }

    override fun onResume() { super.onResume(); handler.post(tick) }
    override fun onPause() {
        handler.removeCallbacks(tick); sticks.release(); handbrake = false; sendInput(); super.onPause()
    }

    override fun onWindowFocusChanged(f: Boolean) {
        super.onWindowFocusChanged(f)
        if (!f) return
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false)
            window.insetsController?.hide(WindowInsets.Type.systemBars())
            window.insetsController?.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        } else {
            @Suppress("DEPRECATION")
            window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
        }
    }
}
