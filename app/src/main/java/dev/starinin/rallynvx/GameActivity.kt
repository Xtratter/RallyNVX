package dev.starinin.rallynvx

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.*
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import io.github.xtratter.uikit.M3
import io.github.xtratter.uikit.M3Surface
import io.github.xtratter.uikit.M3Widgets

/** Free drive: the wheel (or tilt) and the pedals, the handbrake and Reset buttons, sound on/off. */
class GameActivity : Activity(), SurfaceHolder.Callback {
    private var drive: DriveControls? = null
    private var carThr = 0f; private var carSteer = 0f; private var carBrake = false; private var carAmt = 0f
    private var handbrakeBtn: View? = null
    private var hintView: TextView? = null
    private var sensors: android.hardware.SensorManager? = null
    private var tiltListener: android.hardware.SensorEventListener? = null
    private var sentRender = -1
    private var sentAuto = -1
    private var sentRev = -1
    private var needTiltCal = true   // the pose at the start of a drive is the centre (and after "Recalibrate" in the settings)
    private lateinit var hud: Hud
    private lateinit var root: FrameLayout
    private val handler = Handler(Looper.getMainLooper())
    private var running = false
    private var handbrake = false
    private var autopilotOn = false
    private var autopilotStyle = 1
    private var autopilotBtn: View? = null
    private var seed = 0L
    private lateinit var audio: AudioEngine
    private var profile = ""

    private val tick = object : Runnable {
        override fun run() {
            val s = Native.gameStats()
            hud.text = (if (s[27] > 0.5f) t("Raster (auto) · ", "Растр (авто) · ") else "") + "${s[1].toInt()} km/h" + " · ${s[0].toInt()} fps · seed ${s[2].toInt()}" + (if (s[26] > 0.5f) " · " + t("hybrid", "гибрид") else "") + " · " + arrayOf(t("grass", "трава"), t("dirt", "грунт"), t("sand", "песок"), t("rock", "скала"), t("gravel", "гравий"), t("asphalt", "асфальт"), t("water", "вода")).getOrElse(s[28].toInt()) { "" }
            if ((s[29] > 0.5f) != autopilotOn) { autopilotOn = s[29] > 0.5f; showAutopilot() }   // it switches itself off at the finish
            if (autopilotOn && s[29].toInt() - 1 != autopilotStyle) { autopilotStyle = (s[29].toInt() - 1).coerceIn(0, 3); showAutopilot() }
            drive?.speedKmh = s[25]   // signed forward speed: negative while reversing
            handler.postDelayed(this, if (drive != null) 100 else 250)
        }
    }

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        M3.apply(this, M3.Mode.SYSTEM, translucent = true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        seed = intent.getLongExtra("seed", System.nanoTime()) and 0xFFFFFFL   // 24-bit: the HUD shows it exactly
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
        drive = DriveControls(this).also { it.onChange = { thr, steer, brake, amt -> if (autopilotOn && (thr != 0f || steer != 0f || brake)) setAutopilot(false); carThr = thr; carSteer = steer; carBrake = brake; carAmt = amt; sendInput() } }
        drive?.onCamera = { dx, dy -> if (running) Native.gameCamera(dx * 0.006f, -dy * 0.004f) }   // drag right = the camera swings round, drag down = lower
        drive?.onCameraUp = { if (running) Native.gameCameraUp() }
        root.addView(drive, FrameLayout.LayoutParams(-1, -1))
        hud = Hud(this)
        root.addView(hud, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(24f), 0, 0) })
        fun button(label: String, gravity: Int, top: Float, end: Float, w: Float = 120f, onClick: () -> Unit) =
            M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL, onClick).also {
                root.addView(it, FrameLayout.LayoutParams(px(w), px(48f), gravity).apply { setMargins(0, px(top), px(end), 0) })
            }
        button(t("Reset", "Сброс"), Gravity.TOP or Gravity.END, 24f, 16f) { autopilotOn = false; showAutopilot(); Native.gameReset() }
        button(t("New level", "Новый уровень"), Gravity.TOP or Gravity.END, 24f, 150f) { autopilotOn = false; showAutopilot(); seed = System.nanoTime() and 0xFFFFFFL; Native.gameNewLevel(seed) }
        autopilotBtn = button(t("Autopilot", "Автопилот"), Gravity.TOP or Gravity.END, 24f, 284f, 168f) { setAutopilot(!autopilotOn) }
        autopilotBtn?.setOnLongClickListener { cycleAutopilotStyle(); true }   // long press: the next driving style
        button(t("Handbrake", "Ручник"), Gravity.TOP or Gravity.END, 0f, 40f) {}.apply {
            handbrakeBtn = this
            applyControlLayout()
            setOnTouchListener { _, e ->
                when (e.actionMasked) {
                    MotionEvent.ACTION_DOWN -> { if (autopilotOn) setAutopilot(false); handbrake = true; sendInput() }
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

    /** Turns the autopilot on or off (any touch of the wheel or pedals turns it off). */
    private fun setAutopilot(on: Boolean) {
        if (!running) return
        if (on) autopilotStyle = Controls.autopilotStyle(this)
        if (on) sendAutopilotTune()
        autopilotOn = on; Native.gameAutopilot(if (on) autopilotStyle else -1); showAutopilot()
    }
    /** Hands the own style's numbers (settings) to the game. */
    private fun sendAutopilotTune() {
        fun step(key: String, list: FloatArray) = list[Controls.apStep(this, key, list.size)]
        Native.gameAutopilotTune(step("ap_lat", Controls.AP_LAT), step("ap_brake", Controls.AP_BRAKE), step("ap_cut", Controls.AP_CUT), step("ap_speed", Controls.AP_SPEED), Controls.apStep(this, "ap_slide", 2).toFloat(), step("ap_edge", Controls.AP_EDGE))
    }
    /** Long press on the button: calm -> normal -> aggressive -> own -> calm (kept as the setting, applied at once while driving). */
    private fun cycleAutopilotStyle() {
        autopilotStyle = (Controls.autopilotStyle(this) + 1) % Controls.AP_STYLES
        sendAutopilotTune()
        Controls.setAutopilotStyle(this, autopilotStyle)
        if (autopilotOn && running) Native.gameAutopilot(autopilotStyle)
        showAutopilot()
    }
    private fun showAutopilot() {
        val name = arrayOf(t("Calm", "Спокойный"), t("Normal", "Обычный"), t("Aggressive", "Агрессивный"), t("Own", "Свой"))[autopilotStyle.coerceIn(0, 3)]
        (autopilotBtn as? android.widget.TextView)?.text = if (autopilotOn) "● $name" else t("Autopilot", "Автопилот")
        autopilotBtn?.alpha = if (autopilotOn) 1f else 0.85f
    }

    private fun sendInput() {
        if (!running) return
        Native.gameInput(carThr, carSteer, carBrake, handbrake, carAmt)
    }

    override fun surfaceCreated(h: SurfaceHolder) { running = false }
    override fun surfaceChanged(h: SurfaceHolder, f: Int, w: Int, hh: Int) {
        if (running) { Native.gameResize(w, hh); return }
        intent.getIntExtra("lights", -1).let { if (it >= 0) Native.gameLightsDebug(it) }   // debug/test: car lights
        intent.getFloatExtra("tod", -1f).let { if (it >= 0f) Native.gameTimeOfDay(it) }   // debug/test: time of day
        Native.gameStart(h.surface, w, hh, seed, profile, false)
        Native.gameQuality(Quality.index(this))
        Native.gameDust(Controls.dust(this))
        running = true
        sentRender = -1; sentAuto = -1; sentRev = -1; applyRender()   // after `running` is set: the first send must not be skipped
        intent.getIntExtra("hybrid", -1).let { if (it >= 0) { Native.gameRenderMode(it); sentRender = it } }   // debug/test: force a render mode
        intent.getIntExtra("city", 0).let { if (it > 0) Native.gameInCity(it) }   // debug/test: start in a city
        intent.getIntExtra("road", 0).let { if (it > 0) Native.gameOnRoad(it) }   // debug/test: start on a road
        intent.getIntExtra("autopilot", 0).let { if (it > 0) handler.postDelayed({ Controls.setAutopilotStyle(this, it - 1); setAutopilot(true) }, 1500) }   // debug/test: the autopilot drives (1 calm, 2 normal, 3 aggressive)
        // debug/test: a camera orbit (radians) and a held throttle (after 1.5 s) without touching the screen
        val camYaw = intent.getFloatExtra("camyaw", 0f); val camPitch = intent.getFloatExtra("campitch", 0f)
        if (camYaw != 0f || camPitch != 0f) Native.gameCamera(camYaw, camPitch)
        intent.getFloatExtra("steer", 0f).let { st -> if (st != 0f) handler.postDelayed({ carSteer = st; sendInput() }, 1500) }
        intent.getFloatExtra("throttle", 0f).let { th -> if (th != 0f) handler.postDelayed({ carThr = th; sendInput() }, 1500) }
        audio.start()
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

    /** Sends the render mode and the fallback option to a running game (only when they changed). */
    private fun applyRender() {
        if (!running) return
        val mode = Controls.renderMode(this); val auto = if (Controls.autoFallback(this)) 1 else 0
        val rev = Controls.renderRev(this)   // bumped by the settings screen: a deliberate pick is sent again even if the value is the same
        if (mode != sentRender || rev != sentRev) { Native.gameRenderMode(mode); sentRender = mode; sentRev = rev }
        if (auto != sentAuto) { Native.gameAutoFallback(auto == 1); sentAuto = auto }
    }

    private fun applyControls() {
        applyRender()
        Native.audioVolume(Controls.volume(this))
        if (running) Native.gameDust(Controls.dust(this))
        val d = drive ?: return
        d.configure(Controls.mode(this), Controls.sens(this), Controls.pedalsLeft(this), Controls.pedalsAnalog(this))
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
        val pedals = if (side) t("left", "слева") else t("right", "справа")
        val msg = when (mode) {
            Controls.MODE_TILT -> t("Tilt the phone like a wheel to steer. Pedals on the $pedals: gas and brake / reverse.", "Наклоняй телефон как руль. Педали $pedals: газ и тормоз / задний ход.")
            Controls.MODE_WHEEL -> t("Drag the wheel to steer. Pedals on the $pedals: gas (the taller one) and brake / reverse.", "Крути руль пальцем. Педали $pedals: газ (высокая) и тормоз / задний ход.")
            else -> t("Steer with the arrow buttons. Pedals on the $pedals: gas (the taller one) and brake / reverse.", "Рули кнопками-стрелками. Педали $pedals: газ (высокая) и тормоз / задний ход.")
        }
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
        audio.stop(); handler.removeCallbacks(tick); drive?.paused = true; stopTilt(); handbrake = false; sendInput()
        super.onPause()
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
