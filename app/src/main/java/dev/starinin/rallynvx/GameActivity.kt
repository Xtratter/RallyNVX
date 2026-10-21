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
    private var settingsPanel: View? = null
    private var camBtn: View? = null
    private var gearView: GearView? = null
    private var gearBtn: View? = null
    private var reverseBtn: View? = null
    private var carPanel: View? = null
    private var tyreText: TextView? = null
    private var quickBtn: View? = null
    private var carBtn: View? = null
    private var soundBtn: View? = null
    private var menuPanel: View? = null
    private var rallyMode = false
    private var diagBtn: View? = null
    private var loadStartMs = 0L
    private var manual = false
    private var reverseSel = false
    private var todMode = 0   // the quick time-of-day switch (see TimeOfDay); every launch starts on Auto
    private var todBtn: View? = null
    private var weatherBtn: View? = null
    private var camView = 0; private var camDist = 1f; private var camHeight = 0f
    private var seed = 0L
    private lateinit var audio: AudioEngine
    private var profile = ""
    private var rallyIdx = 0   // 1-based rally of the Rally mode, 0 in free drive
    private var rallyStage = 0
    private val stageTotals = FloatArray(RallyData.STAGE_COUNT)
    private var stageAssisted = false
    private var rallyTimer: TextView? = null
    private var rallyCount: TextView? = null
    private var rallyToast: TextView? = null
    private var lastEvent = 0
    private var toastUntil = 0L
    private var finishedShown = false
    private var resultsPanel: FrameLayout? = null
    private var newLevelBtn: View? = null
    private var paceStrip: PaceStrip? = null
    private var loadingView: LoadingView? = null
    private var lastNote = 0
    private var tts: android.speech.tts.TextToSpeech? = null
    private var voicePlayer: VoicePlayer? = null
    private var studioVoice = "system"
    private var voiceLang = "en"
    private var ttsReady = false

    private val tick = object : Runnable {
        override fun run() {
            val s = Native.gameStats()
            hud.text = (if (s[27] > 0.5f) t("Raster (auto) · ", "Растр (авто) · ") else "") + "${s[1].toInt()} km/h" + " · ${s[0].toInt()} fps · seed ${s[2].toInt()}" + (if (s[26] > 0.5f) " · " + t("hybrid", "гибрид") else "") + " · " + arrayOf(t("grass", "трава"), t("dirt", "грунт"), t("sand", "песок"), t("rock", "скала"), t("gravel", "гравий"), t("asphalt", "асфальт"), t("water", "вода")).getOrElse(s[28].toInt()) { "" }
            if ((s[29] > 0.5f) != autopilotOn) { autopilotOn = s[29] > 0.5f; showAutopilot() }   // it switches itself off at the finish
            if (autopilotOn && s[29].toInt() - 1 != autopilotStyle) { autopilotStyle = (s[29].toInt() - 1).coerceIn(0, 3); showAutopilot() }
            if (rallyIdx > 0) updateRally(s)
            if (running) loadingView?.takeIf { it.visibility == View.VISIBLE }?.let { lv ->
                val packed = Native.gameLoadProgress(); lv.report(packed)
                // a start that takes too long, or a dead game thread: say what is known and offer the log to copy
                val slow = System.currentTimeMillis() - loadStartMs > 25000
                if (LoadMath.failed(packed)) lv.setNote(Native.gameLastError()) else if (slow) lv.setNote(t("Slow start: tap the button to copy the log", "Долгая загрузка: нажмите кнопку, чтобы скопировать журнал"))
                if ((slow || LoadMath.failed(packed)) && diagBtn == null) {
                    val act = this@GameActivity; val dpx = resources.displayMetrics.density
                    diagBtn = M3Widgets.button(act, t("Copy the log", "Скопировать журнал"), M3Widgets.ButtonKind.FILLED) {
                        Diag.copy(act, Diag.report(act)); android.widget.Toast.makeText(act, t("Copied: paste it into the chat", "Скопировано: вставьте в чат"), android.widget.Toast.LENGTH_LONG).show()
                    }.also { root.addView(it, FrameLayout.LayoutParams((220f * dpx).toInt(), (52f * dpx).toInt(), Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL).apply { setMargins(0, 0, 0, (40f * dpx).toInt()) }) }
                }
            }
            if (carPanel?.visibility == View.VISIBLE) tyreText?.text = CarMath.tyreLine(floatArrayOf(s[88], s[89], s[90], s[91]), floatArrayOf(s[92], s[93], s[94], s[95]), t("en", "ru") == "en")
            gearView?.update(s[80].toInt(), s[81], s[82] > 0.5f, s[83] > 0.5f, s[86])
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
        rallyIdx = intent.getIntExtra("rally", 0)
        intent.getStringExtra("voice")?.let { Controls.setVoice(this, if (java.util.Locale.getDefault().language == "ru") "ru" else "en", it) }   // debug/test: a studio voice id
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
        drive?.onCamera = { dx, dy -> if (running) { val (sx, sy) = CamMath.dragSigns(Controls.camInvert(this)); Native.gameCamera(dx * 0.006f * sx, -dy * 0.004f * sy) } }   // drag right = the camera swings round, drag down = lower
        drive?.onCameraUp = { if (running) Native.gameCameraUp() }
        drive?.onCameraReset = { changeCamera(1f, 0f); if (running) Native.gameCameraReset() }   // a double tap: the default distance and height, the orbit dropped, the free camera behind the car
        drive?.onCameraPinch = { ratio, dy -> if (CamMath.free(camView)) { if (running) Native.gameCameraFly(CamMath.flyForward(ratio), CamMath.flyUp(dy, resources.displayMetrics.density)) } else changeCamera(CamMath.pinchFor(camView, camDist, ratio), CamMath.height(camView, camHeight, dy, resources.displayMetrics.density)) }   // spread = closer, both fingers up = higher
        drive?.onShift = { d -> if (running) Native.gameShift(d) }
        drive?.onClutch = { p -> if (running) Native.gameClutch(p) }
        manual = Controls.manual(this); drive?.setManual(manual)
        root.addView(drive, FrameLayout.LayoutParams(-1, -1))
        gearView = drive?.cluster   // the instrument cluster stands above the steering buttons (it is part of the controls)
        // the gearbox: a tap switches the automatic and the manual (the clutch pedal and the shift buttons appear); the choice is kept
        gearBtn = M3Widgets.button(this, gearLabel(), M3Widgets.ButtonKind.TONAL) {
            manual = !manual; Controls.setManual(this, manual); drive?.setManual(manual); if (running) Native.gameGearMode(if (manual) 1 else 0)
            (gearBtn as? android.widget.TextView)?.text = gearLabel()
            reverseSel = false; Native.gameReverse(0); updateReverseBtn()
        }.also { root.addView(it, FrameLayout.LayoutParams(px(132f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(76f), px(140f), 0, 0) }) }
        // the automatic's selector: D (drive) or R (reverse): in R the gas pedal drives backwards, so a car rolling back can be pulled out at once
        reverseBtn = M3Widgets.button(this, "D", M3Widgets.ButtonKind.TONAL) {
            reverseSel = !reverseSel; if (running) Native.gameReverse(if (reverseSel) 1 else 0); updateReverseBtn()
        }.also { root.addView(it, FrameLayout.LayoutParams(px(88f), px(64f), Gravity.BOTTOM or Gravity.END)) }
        updateReverseBtn()
        // the car's menu: the driver aids on and off, and the power of every part
        M3Widgets.button(this, t("Car", "Машина"), M3Widgets.ButtonKind.TONAL) { showCarPanel() }
            .also { carBtn = it; root.addView(it, FrameLayout.LayoutParams(px(108f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(280f), px(140f), 0, 0) }) }
        hud = Hud(this)
        root.addView(hud, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(24f), 0, 0) })
        fun button(label: String, gravity: Int, top: Float, end: Float, w: Float = 120f, onClick: () -> Unit) =
            M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL, onClick).also {
                root.addView(it, FrameLayout.LayoutParams(px(w), px(48f), gravity).apply { setMargins(0, px(top), px(end), 0) })
            }
        button(t("Reset", "Сброс"), Gravity.TOP or Gravity.END, 24f, 16f) { autopilotOn = false; showAutopilot(); setHandbrake(false); Native.gameReset() }
        newLevelBtn = button(t("New level", "Новый уровень"), Gravity.TOP or Gravity.END, 24f, 150f) { autopilotOn = false; showAutopilot(); setHandbrake(false); seed = System.nanoTime() and 0xFFFFFFL; Native.gameNewLevel(seed) }
        autopilotBtn = button(t("Autopilot", "Автопилот"), Gravity.TOP or Gravity.END, 24f, 150f, 168f) { setAutopilot(!autopilotOn) }
        autopilotBtn?.setOnLongClickListener { showAutopilotPanel(); true }   // long press: the autopilot settings
        // the camera view button under the status line: a tap cycles the views, a long press puts the current view back to its default distance and height
        intent.getIntExtra("weather", -1).let { if (it >= 0) Controls.setWeather(this, it) }   // debug/test: the weather mode
        Native.gameWeather(Controls.weather(this))   // before the level is made: it starts in that weather
        camView = intent.getIntExtra("camview", Controls.camView(this)).coerceIn(0, CamMath.VIEWS - 1)   // debug/test: `camview`, `camdist`, `camheight` pick a view
        camDist = CamMath.clampDist(intent.getFloatExtra("camdist", Controls.camDist(this, camView))); camHeight = CamMath.clampHeight(camView, intent.getFloatExtra("camheight", Controls.camHeight(this, camView)))
        camBtn = M3Widgets.button(this, camLabel(), M3Widgets.ButtonKind.TONAL) { selectCameraView(CamMath.next(camView)) }.also {
            root.addView(it, FrameLayout.LayoutParams(px(150f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(88f), 0, 0) })
            it.setOnLongClickListener { changeCamera(1f, 0f); true }
        }
        // the quick settings: a gear next to the View button opens a panel of the dust and effect options that applies at once, while driving
        M3Widgets.button(this, "⚙", M3Widgets.ButtonKind.TONAL) { showSettingsPanel() }.also {
            quickBtn = it
            root.addView(it, FrameLayout.LayoutParams(px(56f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(174f), px(88f), 0, 0) })
        }
        // the time of day: a tap cycles Auto, Day, Evening, Night at once (it freezes the sky at that phase; Auto gives the level's own clock back)
        todBtn = M3Widgets.button(this, todLabel(), M3Widgets.ButtonKind.TONAL) {
            todMode = TimeOfDay.next(todMode); Native.gameTimeOfDay(TimeOfDay.phase(todMode)); (todBtn as? android.widget.TextView)?.text = todLabel()
        }.also { root.addView(it, FrameLayout.LayoutParams(px(150f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(240f), px(88f), 0, 0) }) }
        // the weather: a tap cycles Auto, Clear, Cloudy, Rain, Heavy rain, Fog (it blends in over a few seconds; saved like the Settings row)
        weatherBtn = M3Widgets.button(this, weatherLabel(), M3Widgets.ButtonKind.TONAL) {
            Controls.setWeather(this, WeatherMode.next(Controls.weather(this))); Native.gameWeather(Controls.weather(this)); (weatherBtn as? android.widget.TextView)?.text = weatherLabel()
        }.also { root.addView(it, FrameLayout.LayoutParams(px(190f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(398f), px(88f), 0, 0) }) }
        button(t("Handbrake", "Ручник"), Gravity.TOP or Gravity.END, 0f, 40f) {}.apply {
            handbrakeBtn = this
            applyControlLayout()
            setOnTouchListener { _, e ->
                when (e.actionMasked) {
                    // a tap sets the handbrake and the next tap releases it: it stays on for as long as the player wants the car parked
                    MotionEvent.ACTION_DOWN -> { if (autopilotOn) setAutopilot(false); setHandbrake(!handbrake) }
                }
                true
            }
        }
        if (rallyIdx > 0) {
            newLevelBtn?.visibility = View.GONE   // a rally has its fixed level
            fun overlay(size: Float, bold: Boolean) = TextView(this).apply { textSize = size; gravity = Gravity.CENTER; setTextColor(M3.TEXT); if (bold) typeface = M3.medium; visibility = View.GONE }
            rallyTimer = overlay(18f, true).apply { background = M3Surface(this@GameActivity, 20f, M3.surface); setPadding(px(16f), px(8f), px(16f), px(8f)); fontFeatureSettings = "tnum"; visibility = View.VISIBLE }
            root.addView(rallyTimer, FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL).apply { bottomMargin = px(16f) })
            rallyCount = overlay(96f, true).apply { setTextColor(0xFFFFFFFF.toInt()); setShadowLayer(10f, 0f, 3f, 0xDD000000.toInt()) }
            root.addView(rallyCount, FrameLayout.LayoutParams(-2, -2, Gravity.CENTER))
            rallyToast = overlay(18f, true).apply { background = M3Surface(this@GameActivity, 20f, M3.surface); setPadding(px(16f), px(8f), px(16f), px(8f)) }
            paceStrip = PaceStrip(this).apply { visibility = View.GONE }
            root.addView(paceStrip, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply { topMargin = px(21f) })
            val lang = if (java.util.Locale.getDefault().language == "ru") "ru" else "en"
            voiceLang = lang
            studioVoice = Controls.voice(this, lang)
            if (Controls.paceMode(this) == 2 && studioVoice != "system") voicePlayer = VoicePlayer(this)
            // the system synthesizer speaks when it is chosen, when the studio voice is gone, and for the calls the own voice lacks
            if (Controls.paceMode(this) == 2 && (studioVoice == "system" || studioVoice == VoicePlayer.OWN || voicePlayer?.voices()?.none { it.id == studioVoice } == true)) {   // the system speech synthesizer in the device's language
                if (studioVoice != VoicePlayer.OWN) studioVoice = "system"
                val ru = lang == "ru"
                tts = android.speech.tts.TextToSpeech(this) { st ->
                    if (st == android.speech.tts.TextToSpeech.SUCCESS) {
                        val r = tts?.setLanguage(if (ru) java.util.Locale("ru") else java.util.Locale.US)
                        ttsReady = r != null && r >= android.speech.tts.TextToSpeech.LANG_AVAILABLE
                    }
                }
            }
            root.addView(rallyToast, FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL).apply { bottomMargin = px(72f) })
        }
        // sound on/off (kept in SharedPreferences)
        audio = AudioEngine(this)
        val soundB = M3Widgets.button(this, if (audio.muted) "🔇" else "🔊", M3Widgets.ButtonKind.TONAL) {}
        soundBtn = soundB
        soundB.setOnClickListener { audio.muted = !audio.muted; (soundB as? android.widget.TextView)?.text = if (audio.muted) "🔇" else "🔊" }
        root.addView(soundB, FrameLayout.LayoutParams(px(52f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(140f), 0, 0) })
        loadingView = LoadingView(this).also { root.addView(it, FrameLayout.LayoutParams(-1, -1)) }   // over everything until the first frame
        // the settings stay out of the way while driving: one menu button opens them
        menuItems = listOfNotNull(camBtn, quickBtn, todBtn, weatherBtn, soundBtn, gearBtn, carBtn, if (rallyIdx > 0) null else newLevelBtn)
        listOfNotNull(camBtn, quickBtn, todBtn, weatherBtn, soundBtn, gearBtn, carBtn, newLevelBtn).forEach { it.visibility = View.GONE }
        M3Widgets.button(this, "☰", M3Widgets.ButtonKind.TONAL) { showMenuPanel() }
            .also { root.addView(it, FrameLayout.LayoutParams(px(56f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(88f), 0, 0) }) }
        applyControlLayout()
        setContentView(root)
    }

    private var menuItems: List<View> = emptyList()

    /** The menu: every setting that stood on the screen (the view, the quick settings, the time, the weather, the sound, the gearbox, the car, a new level) as one list of buttons. */
    private fun showMenuPanel() {
        menuPanel?.let { it.visibility = View.VISIBLE; (it.tag as? () -> Unit)?.invoke(); return }
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val refreshers = ArrayList<() -> Unit>()
        val colA = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val colB = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        var n = 0
        lateinit var p: FrameLayout
        for (v in menuItems) {
            val tv = v as? TextView ?: continue
            val btn = M3Widgets.button(this, tv.text.toString(), M3Widgets.ButtonKind.TONAL) {}
            val opensPanel = v === quickBtn || v === carBtn || v === newLevelBtn
            btn.setOnClickListener { if (opensPanel) p.visibility = View.GONE; v.performClick(); (btn as? TextView)?.text = tv.text }
            refreshers.add { (btn as? TextView)?.text = tv.text }
            (if (n++ % 2 == 0) colA else colB).addView(btn, LinearLayout.LayoutParams(px(250f), px(50f)).apply { topMargin = px(8f) })
        }
        val list = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; addView(colA, LinearLayout.LayoutParams(px(250f), -2)); addView(colB, LinearLayout.LayoutParams(px(250f), -2).apply { marginStart = px(16f) }) }
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL }
        content.addView(label(t("Menu", "Меню"), 22f))
        content.addView(android.widget.ScrollView(this).apply { addView(list) }, LinearLayout.LayoutParams(-2, px(236f)))
        p = panel(content)
        content.addView(M3Widgets.button(this, t("Close", "Закрыть"), M3Widgets.ButtonKind.FILLED) { p.visibility = View.GONE }, LinearLayout.LayoutParams(px(200f), px(48f)).apply { topMargin = px(8f) })
        p.tag = { refreshers.forEach { it() } }
        p.visibility = View.VISIBLE
        menuPanel = p
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
    /** The handbrake latch: on it holds the car (the rear wheels locked) until the next tap. */
    private fun setHandbrake(on: Boolean) {
        handbrake = on
        (handbrakeBtn as? android.widget.TextView)?.text = HandbrakeMath.label(on, t("en", "ru") == "en")
        sendInput()
    }
    private fun setAutopilot(on: Boolean) {
        if (!running) return
        if (on) autopilotStyle = Controls.autopilotStyle(this)
        if (on) sendAutopilotTune()
        if (on && handbrake) setHandbrake(false)   // the autopilot drives with the handbrake off
        autopilotOn = on; Native.gameAutopilot(if (on) autopilotStyle else -1); showAutopilot()
    }
    /** Hands the own style's numbers (settings) to the game. */
    private fun sendAutopilotTune() {
        fun step(key: String, list: FloatArray) = list[Controls.apStep(this, key, list.size)]
        Native.gameAutopilotTune(step("ap_lat", Controls.AP_LAT), step("ap_brake", Controls.AP_BRAKE), step("ap_cut", Controls.AP_CUT), step("ap_speed", Controls.AP_SPEED), Controls.apStep(this, "ap_slide", 2).toFloat(), step("ap_edge", Controls.AP_EDGE))
    }
    /** Starts (or restarts) a rally stage: 0 the special stage, 1 the city super-special. */
    private fun startRallyStage(stage: Int) {
        rallyStage = stage; finishedShown = false; lastEvent = 0; lastNote = 0
        tts?.stop(); voicePlayer?.stop()
        resultsPanel?.visibility = View.GONE
        autopilotOn = false; showAutopilot()
        Native.gameRally(stage)
    }

    /** Called from the 10 Hz tick with the state array: the timer, the countdown, the event toast and the results. */
    private fun updateRally(s: FloatArray) {
        val phase = s[30].toInt()
        if (phase == 0) return
        val stageText = t("Stage", "Этап") + " ${rallyStage + 1}/${RallyData.STAGE_COUNT}"
        rallyTimer?.text = when (phase) {
            1 -> "$stageText · " + t("get ready", "приготовьтесь")
            else -> "$stageText · ${RallyData.time(s[31])}" + (if (s[32] > 0f) "  +${s[32].toInt()} " + t("s", "с") else "")
        }
        val digit = s[33].toInt()
        rallyCount?.apply {
            when {
                phase == 1 && digit > 0 -> { text = digit.toString(); visibility = View.VISIBLE }
                (phase == 1 && digit == 0) || (phase == 2 && s[31] < 0.8f) -> { text = t("GO!", "СТАРТ!"); visibility = View.VISIBLE }
                else -> visibility = View.GONE
            }
        }
        val serial = s[38].toInt()
        if (serial != lastEvent) {
            lastEvent = serial
            val v = s[39]
            val msg = when (s[37].toInt()) {
                1 -> t("Split ", "Сплит ") + RallyData.time(v)
                2 -> t("Reset  +", "Сброс  +") + "${v.toInt()} " + t("s", "с")
                3 -> t("Off the road  +", "Выезд с дороги  +") + "${v.toInt()} " + t("s", "с")
                4 -> t("Skipped line  +", "Пропущена линия  +") + "${v.toInt()} " + t("s", "с")
                else -> ""
            }
            if (msg.isNotEmpty()) { rallyToast?.text = msg; rallyToast?.visibility = View.VISIBLE; toastUntil = System.currentTimeMillis() + 2500 }
        }
        if (rallyToast?.visibility == View.VISIBLE && System.currentTimeMillis() > toastUntil) rallyToast?.visibility = View.GONE
        if (Controls.paceMode(this) > 0 && phase in 1..2) { paceStrip?.update(s); paceStrip?.visibility = View.VISIBLE } else paceStrip?.visibility = View.GONE
        val note = s[42].toInt()
        if (note > 0 && note != lastNote && phase == 2 && Controls.paceMode(this) > 0) {
            lastNote = note
            val text = Native.gameNoteText(java.util.Locale.getDefault().language == "ru")
            android.util.Log.i("rallynvx", "pace note $note: \"$text\" (voice ${if (ttsReady) "ready" else "not ready"})")
            if (text.isNotEmpty()) {
                if (Controls.paceMode(this) == 2) {
                    val clips = Native.gameNoteClips().split(",").filter { it.isNotEmpty() }
                    if (studioVoice != "system" && voicePlayer?.canPlay(studioVoice, voiceLang, clips) == true) voicePlayer?.play(studioVoice, clips, lang = voiceLang)
                    else if (ttsReady) tts?.speak(text, android.speech.tts.TextToSpeech.QUEUE_FLUSH, null, "note$note")
                }
            }
        }
        if (phase == 3 && !finishedShown) { finishedShown = true; showRallyResults(s[31], s[32], s[41], s[40] > 0.5f) }
    }

    /** The results of a finished stage (and of the rally after the last one) with the records. */
    private fun showRallyResults(time: Float, penalty: Float, total: Float, assisted: Boolean) {
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val rally = rallyIdx - 1
        val record = if (assisted) false else RallyData.saveIfBetter(this, rally, rallyStage, total)
        if (rallyStage == 0) stageAssisted = assisted else stageAssisted = stageAssisted || assisted
        stageTotals[rallyStage] = total
        val last = rallyStage == RallyData.STAGE_COUNT - 1
        val sum = stageTotals.sum()
        val rallyRecord = last && !stageAssisted && stageTotals.all { it > 0f } && RallyData.saveIfBetter(this, rally, -1, sum)
        val stageBest = RallyData.record(this, rally, rallyStage)
        resultsPanel?.let { root.removeView(it) }
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL }
        content.addView(label(t("Stage ", "Этап ") + "${rallyStage + 1}/${RallyData.STAGE_COUNT} " + t("finished", "завершён"), 22f))
        content.addView(label(RallyData.stageName(rallyStage), 15f))
        fun line(text: String) = content.addView(label(text, 16f), LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(6f) })
        line(t("Time ", "Время ") + RallyData.time(time) + (if (penalty > 0f) "  + " + t("penalties ", "штрафы ") + "${penalty.toInt()} " + t("s", "с") else ""))
        line(t("Total ", "Итого ") + RallyData.time(total))
        if (assisted) line(t("Autopilot: not recorded", "Автопилот: не записывается"))
        else if (record) line(t("New record!", "Новый рекорд!"))
        else if (stageBest > 0f) line(t("Record ", "Рекорд ") + RallyData.time(stageBest))
        if (last) {
            line(t("Rally total ", "Итог ралли ") + RallyData.time(sum) + (if (rallyRecord) "  " + t("New record!", "Новый рекорд!") else ""))
        }
        fun add(text: String, kind: M3Widgets.ButtonKind, onClick: () -> Unit) = content.addView(M3Widgets.button(this, text, kind, onClick), LinearLayout.LayoutParams(px(260f), px(48f)).apply { topMargin = px(10f) })
        if (!last) add(t("Next stage", "Следующий этап"), M3Widgets.ButtonKind.FILLED) { startRallyStage(rallyStage + 1) }
        add(t("Retry stage", "Повторить этап"), if (last) M3Widgets.ButtonKind.FILLED else M3Widgets.ButtonKind.TONAL) { startRallyStage(rallyStage) }
        if (last) add(t("Restart the rally", "Ралли заново"), M3Widgets.ButtonKind.TONAL) { startRallyStage(0) }
        add(t("Exit", "Выход"), M3Widgets.ButtonKind.TONAL) { finish() }
        val p = panel(content)
        p.visibility = View.VISIBLE
        resultsPanel = p
    }

    private var autopilotPanel: FrameLayout? = null
    /** Long press on the button: the autopilot settings (style and the own style's numbers), applied at once. */
    private fun showAutopilotPanel() {
        autopilotPanel?.let { it.visibility = View.VISIBLE; (it.tag as? () -> Unit)?.invoke(); return }
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val refreshers = ArrayList<() -> Unit>()
        val list = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        fun changed() { autopilotStyle = Controls.autopilotStyle(this); sendAutopilotTune(); if (autopilotOn && running) Native.gameAutopilot(autopilotStyle); showAutopilot(); refreshers.forEach { it() } }
        fun row(text: String, value: () -> String, onClick: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { this.text = text; textSize = 15f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val btn = M3Widgets.button(this, value(), M3Widgets.ButtonKind.TONAL) {}
            btn.setOnClickListener { onClick(); changed() }
            refreshers.add { (btn as? TextView)?.text = value() }
            r.addView(btn, LinearLayout.LayoutParams(px(150f), px(44f)))
            list.addView(r, LinearLayout.LayoutParams(px(480f), -2).apply { topMargin = px(8f) })
        }
        val names = arrayOf(t("Calm", "Спокойный"), t("Normal", "Обычный"), t("Aggressive", "Агрессивный"), t("Own", "Свой"))
        row(t("Style", "Стиль"), { names[Controls.autopilotStyle(this)] }) { Controls.setAutopilotStyle(this, DriveMath.cycle(Controls.autopilotStyle(this), Controls.AP_STYLES)) }
        fun apRow(text: String, key: String, size: Int, show: (Int) -> String) =
            row(text, { show(Controls.apStep(this, key, size)) }) { Controls.setApStep(this, key, DriveMath.cycle(Controls.apStep(this, key, size), size)) }
        apRow(t("Own: cornering speed", "Свой: скорость в поворотах"), "ap_lat", Controls.AP_LAT.size) { "${(Controls.AP_LAT[it] * 100).toInt()} %" }
        apRow(t("Own: late braking", "Свой: позднее торможение"), "ap_brake", Controls.AP_BRAKE.size) { "${(Controls.AP_BRAKE[it] * 100).toInt()} %" }
        apRow(t("Own: top speed", "Свой: макс. скорость"), "ap_speed", Controls.AP_SPEED.size) { "${(Controls.AP_SPEED[it] * 100).toInt()} %" }
        apRow(t("Own: corner cutting", "Свой: срезание поворотов"), "ap_cut", Controls.AP_CUT.size) { "${(Controls.AP_CUT[it] * 100).toInt()} %" }
        apRow(t("Own: slides", "Свой: заносы"), "ap_slide", 2) { if (it == 1) t("On", "Вкл") else t("Off", "Выкл") }
        apRow(t("Own: margin to the edge", "Свой: запас до края"), "ap_edge", Controls.AP_EDGE.size) { "${Controls.AP_EDGE[it]} ${t("m", "м")}" }
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL }
        content.addView(label(t("Autopilot", "Автопилот"), 22f))
        content.addView(android.widget.ScrollView(this).apply { addView(list) }, LinearLayout.LayoutParams(-2, px(230f)))
        val p = panel(content)
        content.addView(M3Widgets.button(this, t("Close", "Закрыть"), M3Widgets.ButtonKind.FILLED) { p.visibility = View.GONE }, LinearLayout.LayoutParams(px(200f), px(48f)).apply { topMargin = px(12f) })
        p.tag = { refreshers.forEach { it() } }
        p.visibility = View.VISIBLE
        autopilotPanel = p
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
        intent.getIntExtra("aero", -1).let { if (it >= 0) Controls.setAero(this, it > 0) }   // debug/test: the Aerodynamics switch
        intent.getIntExtra("gpuprof", 0).let { if (it > 0) Native.gameGpuProf(it) }   // debug/test: GPU stage timings in the log
        intent.getIntExtra("dustgpu", 0).let { if (it > 0) Native.gameDustGpuDebug(it) }   // debug/test: the dust on the GPU
        intent.getIntExtra("airflow", 0).let { if (it > 0) Native.gameAirflowDebug(it) }   // debug/test: the airflow slice
        Native.gameTimeOfDay(intent.getFloatExtra("tod", -1f))   // debug/test: time of day; a launch without it clears the previous freeze (the switch starts on Auto)
        Native.gameCacheDir(cacheDir.absolutePath)
        loadingView?.restart(); loadStartMs = System.currentTimeMillis(); diagBtn?.let { (it.parent as? android.view.ViewGroup)?.removeView(it) }; diagBtn = null
        Native.gameStart(h.surface, w, hh, seed, profile, false)
        Native.gameQuality(Quality.index(this))
        intent.getIntExtra("dust", -1).let { if (it >= 0) Controls.setDust(this, it) }   // debug/test: the dust level (0 off, 1 normal, 2 lots)
        Native.gameDust(Controls.dust(this))
        Native.gameAero(if (Controls.aero(this)) 1 else 0)
        intent.getIntExtra("blur", -1).let { if (it >= 0) Controls.setMotionBlur(this, it) }   // debug/test: motion blur level
        intent.getIntExtra("dustopts", -1).let { if (it >= 0) Controls.setDustOpts(this, it) }   // debug/test: the GPU dust options as bits
        intent.getIntExtra("shadows", -1).let { if (it >= 0) Controls.setShadows(this, it) }   // debug/test: the shadow quality
        Native.gameShadows(Controls.shadows(this))
        Native.gameMotionBlur(Controls.motionBlur(this))
        Native.gameDustOpts(Controls.dustOpts(this))
        Native.gameSigns(Controls.signs(this))
        Native.gameCameraView(camView, camDist, camHeight)
        Native.gameGearMode(if (manual) 1 else 0)
        Native.gameGearCfg(Controls.boxType(this), Controls.shiftProfile(this))
        Native.gameReverse(if (reverseSel && !manual) 1 else 0)
        Native.gameAssists(Controls.assists(this)); for (i in 0 until CarMath.PARTS) Native.gameTuning(i, Controls.tuning(this, i))
        if (intent.getBooleanExtra("testtrack", false)) Native.gameTestTrack()   // from the main menu: a flat sheet of tarmac to try the sound and the controls
        running = true
        sentRender = -1; sentAuto = -1; sentRev = -1; applyRender()   // after `running` is set: the first send must not be skipped
        intent.getIntExtra("hybrid", -1).let { if (it >= 0) { Native.gameRenderMode(it); sentRender = it } }   // debug/test: force a render mode
        intent.getIntExtra("city", 0).let { if (it > 0) Native.gameInCity(it) }   // debug/test: start in a city
        if (intent.getIntExtra("settingspanel", 0) > 0) handler.postDelayed({ showSettingsPanel() }, 2500)   // debug/test: the quick settings panel
        if (intent.getIntExtra("autopilotpanel", 0) > 0) handler.postDelayed({ showAutopilotPanel() }, 1500)   // debug/test: the autopilot settings
        if (rallyIdx > 0) startRallyStage(intent.getIntExtra("rallystage", 0))   // the Rally mode (debug: `rallystage` 0/1)
        intent.getIntExtra("road", 0).let { if (it > 0) Native.gameOnRoad(it) }   // debug/test: start on a road
        intent.getIntExtra("autopilot", 0).let { if (it > 0) handler.postDelayed({ Controls.setAutopilotStyle(this, it - 1); setAutopilot(true) }, if (rallyIdx > 0) 5500L else 1500L) }   // debug/test: the autopilot drives (1 calm, 2 normal, 3 aggressive)
        // debug/test: a camera orbit (radians) and a held throttle (after 1.5 s) without touching the screen
        val camYaw = intent.getFloatExtra("camyaw", 0f); val camPitch = intent.getFloatExtra("campitch", 0f)
        if (camYaw != 0f || camPitch != 0f) Native.gameCamera(camYaw, camPitch)
        // debug/test: the free camera flies (`camfwd` metres along the gaze, `camup` metres up) and the handbrake starts on
        val camFwd = intent.getFloatExtra("camfwd", 0f); val camUp = intent.getFloatExtra("camup", 0f)
        if (camFwd != 0f || camUp != 0f) Native.gameCameraFly(camFwd, camUp)
        if (intent.getIntExtra("handbrake", 0) > 0) setHandbrake(true)
        intent.getFloatExtra("steer", 0f).let { st -> if (st != 0f) handler.postDelayed({ carSteer = st; sendInput() }, 1500) }
        intent.getFloatExtra("throttle", 0f).let { th -> if (th != 0f) handler.postDelayed({ carThr = th; sendInput() }, 1500) }
        audio.start()
    }
    override fun onDestroy() { tts?.shutdown(); voicePlayer?.release(); super.onDestroy() }
    override fun surfaceDestroyed(h: SurfaceHolder) { audio.stop(); Native.gameStop(); running = false }

    /** Puts the handbrake button above the pedals (on the pedal side set in the settings). */
    private fun applyControlLayout() {
        val dp = resources.displayMetrics.density
        val left = Controls.pedalsLeft(this)
        fun at(w: Float, h: Float, side: Float, bottom: Float) = FrameLayout.LayoutParams((w * dp).toInt(), (h * dp).toInt(), Gravity.BOTTOM or (if (left) Gravity.START else Gravity.END))
            .apply { val s = (side * dp).toInt(); val bt = (bottom * dp).toInt(); if (left) setMargins(s, 0, 0, bt) else setMargins(0, 0, s, bt) }
        // the pedals stand at the edge: the handbrake above the brake, the D / R selector where the clutch is (the automatic has none)
        handbrakeBtn?.layoutParams = at(100f, 48f, 86f, 138f)
        reverseBtn?.layoutParams = at(88f, 64f, 194f, 14f)
    }

    /** Sends the render mode and the fallback option to a running game (only when they changed). */
    private fun applyRender() {
        if (!running) return
        val mode = Controls.renderMode(this); val auto = if (Controls.autoFallback(this)) 1 else 0
        val rev = Controls.renderRev(this)   // bumped by the settings screen: a deliberate pick is sent again even if the value is the same
        if (mode != sentRender || rev != sentRev) { Native.gameRenderMode(mode); sentRender = mode; sentRev = rev }
        if (auto != sentAuto) { Native.gameAutoFallback(auto == 1); sentAuto = auto }
    }

    /** The quick settings over the game: the dust, its four physics options, aerodynamics, motion blur and the camera view; every change applies at once and is kept. */
    private fun showSettingsPanel() {
        settingsPanel?.let { it.visibility = View.VISIBLE; (it.tag as? () -> Unit)?.invoke(); return }
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val refreshers = ArrayList<() -> Unit>()
        // two columns so all the rows are in view at once
        val colA = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val colB = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val list = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; addView(colA, LinearLayout.LayoutParams(px(330f), -2)); addView(colB, LinearLayout.LayoutParams(px(330f), -2).apply { marginStart = px(16f) }) }
        var rows = 0
        fun applyLive() { if (running) { Native.gameDust(Controls.dust(this)); Native.gameAero(if (Controls.aero(this)) 1 else 0); Native.gameDustOpts(Controls.dustOpts(this)); Native.gameMotionBlur(Controls.motionBlur(this)); Native.gameShadows(Controls.shadows(this)); Native.gameWeather(Controls.weather(this)) }; refreshers.forEach { it() } }
        fun row(text: String, value: () -> String, onClick: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { this.text = text; textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val btn = M3Widgets.button(this, value(), M3Widgets.ButtonKind.TONAL) {}
            btn.setOnClickListener { onClick(); applyLive() }
            refreshers.add { (btn as? TextView)?.text = value() }
            r.addView(btn, LinearLayout.LayoutParams(px(120f), px(44f)))
            (if (rows++ % 2 == 0) colA else colB).addView(r, LinearLayout.LayoutParams(px(330f), -2).apply { topMargin = px(8f) })
        }
        fun onOff(b: Boolean) = if (b) t("On", "Вкл") else t("Off", "Выкл")
        row(t("Dust", "Пыль"), { arrayOf(t("Off", "Выкл"), t("Normal", "Обычная"), t("Lots", "Много"))[Controls.dust(this)] }) { Controls.setDust(this, DriveMath.cycle(Controls.dust(this), 3)) }
        row(t("Aerodynamics", "Аэродинамика"), { onOff(Controls.aero(this)) }) { Controls.setAero(this, !Controls.aero(this)) }
        row(t("Particle pressure", "Давление частиц"), { onOff(Controls.dustOpt(this, 1)) }) { Controls.toggleDustOpt(this, 1) }
        row(t("Exact car shape", "Точная форма кузова"), { onOff(Controls.dustOpt(this, 2)) }) { Controls.toggleDustOpt(this, 2) }
        row(t("World collisions", "Столкновения с миром"), { onOff(Controls.dustOpt(this, 4)) }) { Controls.toggleDustOpt(this, 4) }
        row(t("Settled dust layer", "Слой осевшей пыли"), { onOff(Controls.dustOpt(this, 8)) }) { Controls.toggleDustOpt(this, 8) }
        row(t("Motion blur", "Размытие в движении"), { arrayOf(t("Off", "Выкл"), t("Low", "Слабое"), t("High", "Сильное"))[Controls.motionBlur(this)] }) { Controls.setMotionBlur(this, DriveMath.cycle(Controls.motionBlur(this), 3)) }
        row(t("Camera drag inversion", "Инверсия поворота камеры"), { CamMath.invertName(Controls.camInvert(this), java.util.Locale.getDefault().language != "ru") }) { Controls.setCamInvert(this, DriveMath.cycle(Controls.camInvert(this), CamMath.INVERT_MODES)) }
        row(t("Camera view", "Вид камеры"), { CamMath.name(camView, java.util.Locale.getDefault().language != "ru") }) { selectCameraView(CamMath.next(camView)) }
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL }
        content.addView(label(t("Quick settings", "Быстрые настройки"), 22f))
        content.addView(android.widget.ScrollView(this).apply { addView(list) }, LinearLayout.LayoutParams(-2, px(230f)))
        val p = panel(content)
        content.addView(M3Widgets.button(this, t("Close", "Закрыть"), M3Widgets.ButtonKind.FILLED) { p.visibility = View.GONE }, LinearLayout.LayoutParams(px(200f), px(48f)).apply { topMargin = px(12f) })
        p.tag = { refreshers.forEach { it() } }
        p.visibility = View.VISIBLE
        settingsPanel = p
    }

    /** The car's menu: on the left the aids (a tap switches each on or off), on the right the power of every part (steps of 10 %, the tuning's range in the engine). */
    private fun showCarPanel() {
        carPanel?.let { it.visibility = View.VISIBLE; (it.tag as? () -> Unit)?.invoke(); return }
        val dp = resources.displayMetrics.density
        fun px(v: Float) = (v * dp).toInt()
        val en = t("en", "ru") == "en"
        val refreshers = ArrayList<() -> Unit>()
        val colA = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val colB = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        colA.addView(label(t("Driver aids", "Системы"), 17f)); colB.addView(label(t("Power of the parts", "Мощность узлов"), 17f))
        for (bit in 0 until CarMath.AIDS) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = CarMath.aidName(bit, en); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            fun lab() = if (CarMath.aidOn(Controls.assists(this), bit)) t("On", "Вкл") else t("Off", "Выкл")
            val b = M3Widgets.button(this, lab(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener { val m = CarMath.toggle(Controls.assists(this), bit); Controls.setAssists(this, m); if (running) Native.gameAssists(m); (b as? TextView)?.text = lab() }
            refreshers.add { (b as? TextView)?.text = lab() }
            r.addView(b, LinearLayout.LayoutParams(px(90f), px(42f)))
            colA.addView(r, LinearLayout.LayoutParams(px(330f), -2).apply { topMargin = px(6f) })
        }
        // the gearbox: the manual's type and the automatic's character; one button each, a tap goes to the next
        fun cycleRow(title: String, value: () -> String, onTap: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = title; textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val b = M3Widgets.button(this, value(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener { onTap(); (b as? TextView)?.text = value() }
            refreshers.add { (b as? TextView)?.text = value() }
            r.addView(b, LinearLayout.LayoutParams(px(150f), px(42f)))
            colA.addView(r, LinearLayout.LayoutParams(px(330f), -2).apply { topMargin = px(6f) })
        }
        cycleRow(t("Manual box", "Ручная коробка"), { GearMath.boxName(Controls.boxType(this), en) }) { Controls.setBoxType(this, GearMath.next(Controls.boxType(this), GearMath.BOXES)); sendGearCfg() }
        cycleRow(t("Automatic", "Автомат"), { GearMath.profileName(Controls.shiftProfile(this), en) }) { Controls.setShiftProfile(this, GearMath.next(Controls.shiftProfile(this), GearMath.PROFILES)); sendGearCfg() }
        run {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = t("Differentials", "Дифференциалы"); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            fun lab() = CarMath.presetName(Controls.diffPreset(this), en)
            val b = M3Widgets.button(this, lab(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener {
                val n = GearMath.next(Controls.diffPreset(this), CarMath.PRESET_COUNT); Controls.setDiffPreset(this, n)
                for (k in 0 until 4) { Controls.setTuning(this, 7 + k, CarMath.PRESETS[n][k]); if (running) Native.gameTuning(7 + k, CarMath.PRESETS[n][k]) }
                (b as? TextView)?.text = lab(); refreshers.forEach { it() }
            }
            refreshers.add { (b as? TextView)?.text = lab() }
            r.addView(b, LinearLayout.LayoutParams(px(110f), px(42f)))
            colB.addView(r, LinearLayout.LayoutParams(px(300f), -2).apply { topMargin = px(6f) })
        }
        run {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = t("Suspension", "Подвеска"); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            fun lab() = CarMath.chassisPresetName(Controls.chassisPreset(this), en)
            val b = M3Widgets.button(this, lab(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener {
                val n = GearMath.next(Controls.chassisPreset(this), CarMath.CHASSIS_PRESET_COUNT); Controls.setChassisPreset(this, n)
                for (k in 0 until 6) { Controls.setTuning(this, 13 + k, CarMath.CHASSIS_PRESETS[n][k]); if (running) Native.gameTuning(13 + k, CarMath.CHASSIS_PRESETS[n][k]) }
                (b as? TextView)?.text = lab(); refreshers.forEach { it() }
            }
            refreshers.add { (b as? TextView)?.text = lab() }
            r.addView(b, LinearLayout.LayoutParams(px(110f), px(42f)))
            colB.addView(r, LinearLayout.LayoutParams(px(300f), -2).apply { topMargin = px(6f) })
        }
        for (i in 0 until CarMath.PARTS) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = CarMath.partName(i, en); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val v = TextView(this).apply { textSize = 14f; setTextColor(M3.TEXT); gravity = Gravity.CENTER }
            fun show() { v.text = CarMath.valueText(i, Controls.tuning(this, i)) }
            fun change(dir: Int) { val nv = CarMath.step(i, Controls.tuning(this, i), dir); Controls.setTuning(this, i, nv); if (running) Native.gameTuning(i, nv); show() }
            show(); refreshers.add { show() }
            r.addView(M3Widgets.button(this, "−", M3Widgets.ButtonKind.TONAL) { change(-1) }, LinearLayout.LayoutParams(px(44f), px(42f)))
            r.addView(v, LinearLayout.LayoutParams(px(64f), -2))
            r.addView(M3Widgets.button(this, "+", M3Widgets.ButtonKind.TONAL) { change(1) }, LinearLayout.LayoutParams(px(44f), px(42f)))
            colB.addView(r, LinearLayout.LayoutParams(px(300f), -2).apply { topMargin = px(6f) })
        }
        val list = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; addView(colA, LinearLayout.LayoutParams(px(330f), -2)); addView(colB, LinearLayout.LayoutParams(px(300f), -2).apply { marginStart = px(20f) }) }
        val content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL }
        content.addView(label(t("Car", "Машина"), 22f))
        content.addView(TextView(this).apply { textSize = 13f; setTextColor(M3.TEXT); gravity = Gravity.CENTER; tyreText = this }, LinearLayout.LayoutParams(-2, -2).apply { topMargin = px(4f) })   // the tyres: temperature and wear, live while the panel is open
        content.addView(android.widget.ScrollView(this).apply { addView(list) }, LinearLayout.LayoutParams(-2, px(250f)))
        val p = panel(content)
        content.addView(M3Widgets.button(this, t("Reset tuning", "Сбросить настройки"), M3Widgets.ButtonKind.TONAL) {
            for (i in 0 until CarMath.PARTS) { Controls.setTuning(this, i, 1f); if (running) Native.gameTuning(i, 1f) }
            Controls.setDiffPreset(this, 1); Controls.setChassisPreset(this, 0)
            Controls.setAssists(this, CarMath.ALL_AIDS); if (running) Native.gameAssists(CarMath.ALL_AIDS); refreshers.forEach { it() }
        }, LinearLayout.LayoutParams(px(240f), px(44f)).apply { topMargin = px(8f) })
        content.addView(M3Widgets.button(this, t("Close", "Закрыть"), M3Widgets.ButtonKind.FILLED) { p.visibility = View.GONE }, LinearLayout.LayoutParams(px(200f), px(48f)).apply { topMargin = px(8f) })
        p.tag = { refreshers.forEach { it() } }
        p.visibility = View.VISIBLE
        carPanel = p
    }

    private fun weatherLabel() = t("Weather: ", "Погода: ") + WeatherMode.name(Controls.weather(this), java.util.Locale.getDefault().language != "ru")
    private fun todLabel() = t("Time: ", "Время: ") + TimeOfDay.name(todMode, java.util.Locale.getDefault().language != "ru")
    private fun camLabel() = t("View: ", "Вид: ") + CamMath.name(camView, java.util.Locale.getDefault().language != "ru")

    /** Takes the distance (zoom) and height (tilt) of the current view, keeps them for next time and sends them to the game. */
    private fun updateReverseBtn() {
        (reverseBtn as? android.widget.TextView)?.text = if (reverseSel) "R" else "D"
        reverseBtn?.visibility = if (manual) View.GONE else View.VISIBLE
        reverseBtn?.alpha = if (reverseSel) 1f else 0.85f
    }

    private fun sendGearCfg() { if (running) Native.gameGearCfg(Controls.boxType(this), Controls.shiftProfile(this)) }

    private fun gearLabel() = if (manual) t("Gearbox: manual", "Коробка: МКПП") else t("Gearbox: auto", "Коробка: АКПП")

    private fun changeCamera(dist: Float, height: Float) {
        camDist = CamMath.clampDist(dist); camHeight = CamMath.clampHeight(camView, height)
        Controls.setCam(this, camView, camDist, camHeight)
        if (running) Native.gameCameraView(camView, camDist, camHeight)
    }

    private fun selectCameraView(view: Int) {
        camView = view; camDist = Controls.camDist(this, view); camHeight = Controls.camHeight(this, view)
        Controls.setCam(this, camView, camDist, camHeight)
        (camBtn as? android.widget.TextView)?.text = camLabel()
        if (running) Native.gameCameraView(camView, camDist, camHeight)
    }

    private fun applyControls() {
        applyRender()
        Native.audioVolume(Controls.volume(this))
        if (running) { Native.gameDust(Controls.dust(this)); Native.gameAero(if (Controls.aero(this)) 1 else 0); Native.gameMotionBlur(Controls.motionBlur(this)); Native.gameShadows(Controls.shadows(this)); Native.gameDustOpts(Controls.dustOpts(this)) }
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
        audio.stop(); handler.removeCallbacks(tick); drive?.paused = true; stopTilt(); setHandbrake(false)
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
