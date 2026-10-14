package dev.starinin.raytrace

import android.app.Activity
import android.content.res.ColorStateList
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.*
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Surface
import io.github.xtratter.uikit.M3Widgets

/**
 * Free drive or a mission (extra "mission" >= 0): left stick = steer/throttle, right stick down = brake.
 * Mission HUD: timer, cargo integrity, arrow and distance to the target; pause and results panels.
 */
class GameActivity : Activity(), SurfaceHolder.Callback {
    private var sticks: SticksLayer? = null
    private var drive: DriveControls? = null
    private var carThr = 0f; private var carSteer = 0f; private var carBrake = false; private var carAmt = 0f
    private var handbrakeBtn: View? = null
    private var hintView: TextView? = null
    private var sensors: android.hardware.SensorManager? = null
    private var tiltListener: android.hardware.SensorEventListener? = null
    private var needTiltCal = true   // the pose at the start of a drive is the centre (and after "Recalibrate" in the settings)
    private lateinit var hud: Hud
    private lateinit var root: FrameLayout
    private val handler = Handler(Looper.getMainLooper())
    private var running = false
    private var handbrake = false
    private var lx = 0f; private var ly = 0f; private var ry = 0f
    private var seed = 0L
    private var mission = -1
    private lateinit var audio: AudioEngine
    private var air = false
    private var rx = 0f
    private var profile = ""

    private var timer: Hud? = null
    private var bar: ProgressBar? = null
    private var arrow: ArrowView? = null
    private var dist: Hud? = null
    private var pausePanel: View? = null
    private var resultPanel: FrameLayout? = null
    private var resultShown = false
    private var ignoreRun = -1f   // run counter ([18]) whose stale result must not reopen the panel after Retry

    private val tick = object : Runnable {
        override fun run() {
            val s = Native.gameStats()
            hud.text = (if (s[20] > 0f) "${s[1].toInt()} km/h · ${s[19].toInt()} m" else "${s[1].toInt()} km/h") + " · ${s[0].toInt()} fps · seed ${s[2].toInt()}"
            if (mission >= 0) updateMission(s)
            drive?.speedKmh = s[25]   // signed forward speed: negative while reversing
            handler.postDelayed(this, if (mission >= 0 || drive != null) 100 else 250)
        }
    }

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        seed = intent.getLongExtra("seed", System.nanoTime()) and 0xFFFFFFL   // 24-bit: the HUD shows it exactly
        mission = intent.getIntExtra("mission", -1)
        air = intent.getBooleanExtra("air", false)
        profile = intent.getStringExtra("profile") ?: ""
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        root = FrameLayout(this)
        val sv = SurfaceView(this)
        sv.holder.addCallback(this)
        val scale = Quality.scale(this)
        if (scale < 0.999f) {
            val dm = resources.displayMetrics
            sv.holder.setFixedSize((dm.widthPixels * scale).toInt().coerceAtLeast(2), (dm.heightPixels * scale).toInt().coerceAtLeast(2))
        }
        root.addView(sv, FrameLayout.LayoutParams(-1, -1))
        if (air) {
            sticks = SticksLayer(this).also { it.onChange = { x, y, rX, rY -> lx = x; ly = y; rx = rX; ry = rY; sendInput() } }
            root.addView(sticks, FrameLayout.LayoutParams(-1, -1))
        } else {
            drive = DriveControls(this).also { it.onChange = { thr, steer, brake, amt -> carThr = thr; carSteer = steer; carBrake = brake; carAmt = amt; sendInput() } }
            root.addView(drive, FrameLayout.LayoutParams(-1, -1))
        }
        hud = Hud(this)
        root.addView(hud, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(24f), 0, 0) })
        fun button(label: String, gravity: Int, top: Float, end: Float, w: Float = 120f, onClick: () -> Unit) =
            M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL, onClick).also {
                root.addView(it, FrameLayout.LayoutParams(px(w), px(48f), gravity).apply { setMargins(0, px(top), px(end), 0) })
            }
        if (!air) button(t("Reset", "Сброс"), Gravity.TOP or Gravity.END, 24f, 16f) { Native.gameReset() }
        if (mission < 0) button(t("New level", "Новый уровень"), Gravity.TOP or Gravity.END, 24f, 150f) { seed = System.nanoTime() and 0xFFFFFFL; Native.gameNewLevel(seed) }
        if (!air) button(t("Handbrake", "Ручник"), Gravity.TOP or Gravity.END, 0f, 40f) {}.apply {
            handbrakeBtn = this
            applyControlLayout()
            setOnTouchListener { _, e ->
                when (e.actionMasked) {
                    MotionEvent.ACTION_DOWN -> { handbrake = true; sendInput() }
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> { handbrake = false; sendInput() }
                }
                true
            }
        }
        // sound on/off (kept in SharedPreferences)
        audio = AudioEngine(this)
        val soundBtn = M3Widgets.button(this, if (audio.muted) "🔇" else "🔊", M3Widgets.ButtonKind.TONAL) {}
        soundBtn.setOnClickListener { audio.muted = !audio.muted; (soundBtn as? android.widget.TextView)?.text = if (audio.muted) "🔇" else "🔊" }
        root.addView(soundBtn, FrameLayout.LayoutParams(px(52f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(140f), 0, 0) })
        if (mission >= 0) buildMissionUi(::px)
        setContentView(root)
    }

    private fun panel(content: LinearLayout): FrameLayout {
        val dp = resources.displayMetrics.density
        val f = FrameLayout(this).apply { setBackgroundColor(0xAA000000.toInt()); visibility = View.GONE; isClickable = true }
        content.background = M3Surface(this, 24f, M3.surface)
        content.setPadding((24 * dp).toInt(), (20 * dp).toInt(), (24 * dp).toInt(), (20 * dp).toInt())
        f.addView(content, FrameLayout.LayoutParams(-2, -2, Gravity.CENTER))
        root.addView(f, FrameLayout.LayoutParams(-1, -1))
        return f
    }

    private fun label(text: String, size: Float = 16f) = TextView(this).apply { this.text = text; textSize = size; setTextColor(M3.TEXT); gravity = Gravity.CENTER }

    private fun buildMissionUi(px: (Float) -> Int) {
        timer = Hud(this).also { root.addView(it, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply { setMargins(0, px(24f), 0, 0) }) }
        bar = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply {
            max = 100; progress = 100; progressTintList = ColorStateList.valueOf(M3.primary)
            root.addView(this, FrameLayout.LayoutParams(px(160f), px(8f), Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply { setMargins(0, px(70f), 0, 0) })
        }
        val nav = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        arrow = ArrowView(this).also { nav.addView(it) }
        dist = Hud(this).also { nav.addView(it) }
        root.addView(nav, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply { setMargins(0, px(84f), 0, 0) })
        M3Widgets.button(this, "II", M3Widgets.ButtonKind.TONAL) { showPause() }.also {
            root.addView(it, FrameLayout.LayoutParams(px(52f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(84f), 0, 0) })
        }
        // pause panel
        val pc = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER }
        pc.addView(label(t("Paused", "Пауза"), 22f))
        fun pbtn(text: String, onClick: () -> Unit) = pc.addView(M3Widgets.button(this, text, M3Widgets.ButtonKind.TONAL, onClick), LinearLayout.LayoutParams(px(220f), px(48f)).apply { topMargin = px(10f) })
        pbtn(t("Resume", "Продолжить")) { hidePause() }
        fun soundLabel() = if (audio.muted) t("Sound: off", "Звук: выкл") else t("Sound: on", "Звук: вкл")
        val soundPause = M3Widgets.button(this, soundLabel(), M3Widgets.ButtonKind.TONAL) {}
        soundPause.setOnClickListener { audio.muted = !audio.muted; (soundPause as? TextView)?.text = soundLabel() }
        pc.addView(soundPause, LinearLayout.LayoutParams(px(220f), px(48f)).apply { topMargin = px(10f) })
        pbtn(t("Settings", "Настройки")) { startActivity(android.content.Intent(this, SettingsActivity::class.java)) }
        pbtn(t("Restart", "Заново")) { ignoreRun = Native.gameStats()[18]; Native.gameRestart(); resultShown = false; hidePause() }
        pbtn(t("Abandon", "Бросить")) { Native.gameAbort(); finish() }
        pausePanel = panel(pc)
        // result panel (content filled when the result arrives)
        resultPanel = panel(LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER })
    }

    private fun showPause() { if (resultShown) return; Native.gamePause(true); pausePanel?.visibility = View.VISIBLE }
    private fun hidePause() { pausePanel?.visibility = View.GONE; Native.gamePause(false) }

    private fun updateMission(s: FloatArray) {
        val phase = s[4].toInt()
        if (phase == 0) return
        val left = (s[6] - s[5]).coerceAtLeast(0f)
        val kind = s[17].toInt()
        timer?.text = (if (phase == 1) t("Go!", "Вперёд!") else "%d:%02d".format(left.toInt() / 60, left.toInt() % 60)) +
            (if (kind == 1 || kind == 2) "  ·  ${minOf(s[10].toInt() + 1, s[11].toInt())}/${s[11].toInt()}" else if (kind == 4) "  ·  ${s[10].toInt()}/${s[11].toInt()}" else if (kind == 5) "  ·  ${s[10].toInt()}/${(s[11].toInt() - 1).coerceAtLeast(1)}  ·  👥 ${s[23].toInt()}" else "")
        bar?.visibility = if (kind == 0 || kind == 3 || kind == 5) View.VISIBLE else View.GONE
        bar?.progress = s[7].toInt()
        arrow?.setAngle(s[9]); dist?.text = (if (s[21] > 0.5f) "📦 " else "") + "${s[8].toInt()} m" + (if (s[24] > 0f) "  ·  ${(s[24] * 100).toInt()}%" else "")
        if (s[12] != 0f && !resultShown && s[18] != ignoreRun) showResult(s)
        if (s[12] == 0f && resultShown) { resultShown = false; resultPanel?.visibility = View.GONE }
    }

    private fun showResult(s: FloatArray) {
        resultShown = true
        pausePanel?.visibility = View.GONE
        val p = resultPanel ?: return
        val c = p.getChildAt(0) as LinearLayout
        c.removeAllViews()
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        if (s[12] == 1f) {
            c.addView(label(arrayOf("", "🥉", "🥈", "🥇")[s[13].toInt().coerceIn(0, 3)] + "  " + t("Mission complete", "Миссия выполнена"), 22f))
            c.addView(label(t("Time: ", "Время: ") + "%d:%02d".format(s[5].toInt() / 60, s[5].toInt() % 60) + if (s[17] == 0f || s[17] == 3f || s[17] == 5f) "   " + t("Cargo: ", "Груз: ") + "${s[7].toInt()}%" else ""))
            c.addView(label(t("Reward: ", "Награда: ") + "+${s[14].toInt()} $   ·   " + t("Money: ", "Деньги: ") + "${s[16].toInt()} $"))
        } else {
            val why = when (s[15].toInt()) { 1 -> t("Out of time", "Время вышло"); 2 -> if (s[17].toInt() == 5) t("Passengers hurt", "Пассажиры пострадали") else t("Cargo destroyed", "Груз разбит"); 4 -> t("Crashed", "Авария"); else -> t("Abandoned", "Миссия брошена") }
            c.addView(label(t("Failed", "Провал") + ": " + why, 22f))
        }
        c.addView(M3Widgets.button(this, t("Retry", "Ещё раз"), M3Widgets.ButtonKind.FILLED) { ignoreRun = Native.gameStats()[18]; Native.gameRestart(); resultShown = false; p.visibility = View.GONE },
            LinearLayout.LayoutParams(px(220f), px(48f)).apply { topMargin = px(12f) })
        c.addView(M3Widgets.button(this, t("To missions", "К миссиям"), M3Widgets.ButtonKind.TONAL) { finish() },
            LinearLayout.LayoutParams(px(220f), px(48f)).apply { topMargin = px(8f) })
        p.visibility = View.VISIBLE
    }

    private fun sendInput() {
        if (!running) return
        if (air) Native.gameFlight(lx, ly, ry, rx) else Native.gameInput(carThr, carSteer, carBrake, handbrake, carAmt)
    }

    override fun surfaceCreated(h: SurfaceHolder) { running = false }
    override fun surfaceChanged(h: SurfaceHolder, f: Int, w: Int, hh: Int) {
        if (running) { Native.gameResize(w, hh); return }
        Native.gameStart(h.surface, w, hh, seed, profile, mission)
        Native.gameQuality(Quality.index(this))
        intent.getIntExtra("city", 0).let { if (it > 0) Native.gameInCity(it) }   // debug/test: start in a city
        running = true
        audio.start()
        // the engine restarts after a surface loss (screen lock): stay paused if the pause panel is up
        if (mission >= 0 && pausePanel?.visibility == View.VISIBLE) Native.gamePause(true)
    }
    override fun surfaceDestroyed(h: SurfaceHolder) { audio.stop(); Native.gameStop(); running = false }

    /** Puts the handbrake button above the pedals (on the pedal side set in the settings). */
    private fun applyControlLayout() {
        val b = handbrakeBtn ?: return
        val dp = resources.displayMetrics.density
        val left = Controls.pedalsLeft(this)
        b.layoutParams = FrameLayout.LayoutParams((120f * dp).toInt(), (48f * dp).toInt(), Gravity.BOTTOM or (if (left) Gravity.START else Gravity.END))
            .apply { if (left) setMargins((28f * dp).toInt(), 0, 0, (214f * dp).toInt()) else setMargins(0, 0, (28f * dp).toInt(), (214f * dp).toInt()) }
    }

    private fun applyControls() {
        Native.audioVolume(Controls.volume(this))
        val d = drive ?: return
        d.configure(Controls.mode(this), Controls.sens(this), Controls.pedalsLeft(this))
        applyControlLayout()
        // tilt steering: the pose at resume (or after "Calibrate" in the settings) is the centre
        stopTilt()
        if (Controls.mode(this) == Controls.MODE_TILT) startTilt(d)
        showHintOnce()
    }

    private fun startTilt(d: DriveControls) {
        val sm = getSystemService(SENSOR_SERVICE) as android.hardware.SensorManager
        val sensor = sm.getDefaultSensor(android.hardware.Sensor.TYPE_GRAVITY) ?: sm.getDefaultSensor(android.hardware.Sensor.TYPE_ACCELEROMETER) ?: return
        val l = object : android.hardware.SensorEventListener {
            override fun onSensorChanged(e: android.hardware.SensorEvent) {
                val x = e.values[0]; val y = e.values[1]
                // roll of the phone about the screen normal, clockwise positive, for the current display rotation
                val phi = Math.toDegrees(when (windowManager.defaultDisplay.rotation) {
                    android.view.Surface.ROTATION_90 -> Math.atan2(y.toDouble(), x.toDouble())
                    android.view.Surface.ROTATION_270 -> Math.atan2(-y.toDouble(), -x.toDouble())
                    android.view.Surface.ROTATION_180 -> Math.atan2(x.toDouble(), -y.toDouble())
                    else -> Math.atan2(-x.toDouble(), y.toDouble())
                }).toFloat()
                d.tiltRoll = phi
                if (needTiltCal || Controls.wantsCalibration(this@GameActivity)) { d.calibrate(); needTiltCal = false; Controls.setWantsCalibration(this@GameActivity, false) }
                d.tiltLive = true
            }
            override fun onAccuracyChanged(s: android.hardware.Sensor?, a: Int) {}
        }
        sm.registerListener(l, sensor, android.hardware.SensorManager.SENSOR_DELAY_GAME)
        sensors = sm; tiltListener = l
    }

    private fun stopTilt() { tiltListener?.let { sensors?.unregisterListener(it) }; tiltListener = null; drive?.tiltLive = false }

    private fun showHintOnce() {
        val mode = Controls.mode(this)
        if (drive == null || Controls.hintSeen(this, mode)) return
        Controls.setHintSeen(this, mode, true)
        val dp = resources.displayMetrics.density
        val side = Controls.pedalsLeft(this)
        val msg = if (mode == Controls.MODE_TILT)
            t("Tilt the phone like a wheel to steer. Pedals on the ${if (side) "left" else "right"}: gas and brake / reverse.", "Наклоняй телефон как руль. Педали ${if (side) "слева" else "справа"}: газ и тормоз / задний ход.")
        else t("Drag the wheel to steer. Pedals on the ${if (side) "left" else "right"}: gas (the taller one) and brake / reverse.", "Крути руль пальцем. Педали ${if (side) "слева" else "справа"}: газ (высокая) и тормоз / задний ход.")
        hintView?.let { root.removeView(it) }
        val tv = TextView(this).apply {
            text = msg; textSize = 15f; setTextColor(M3.TEXT); gravity = Gravity.CENTER
            setPadding((16f * dp).toInt(), (10f * dp).toInt(), (16f * dp).toInt(), (10f * dp).toInt())
            background = M3Surface(this@GameActivity, 16f, M3.surface)
        }
        hintView = tv
        root.addView(tv, FrameLayout.LayoutParams(-2, -2, Gravity.CENTER_HORIZONTAL or Gravity.BOTTOM).apply { setMargins(0, 0, 0, (250f * dp).toInt()) })
        tv.postDelayed({ root.removeView(tv); if (hintView === tv) hintView = null }, 6000)
    }

    override fun onResume() { super.onResume(); handler.post(tick); if (running) audio.start(); drive?.paused = false; applyControls() }
    override fun onPause() {
        audio.stop(); handler.removeCallbacks(tick); sticks?.release(); drive?.paused = true; stopTilt(); handbrake = false; sendInput()
        if (mission >= 0 && !resultShown) showPause()
        super.onPause()
    }

    @Deprecated("Deprecated in Java")
    override fun onBackPressed() {
        when {
            mission < 0 || resultShown -> super.onBackPressed()
            pausePanel?.visibility == View.VISIBLE -> hidePause()
            else -> showPause()
        }
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
