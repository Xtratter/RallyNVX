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
        drive?.onCameraPinch = { ratio, dy -> changeCamera(CamMath.pinchFor(camView, camDist, ratio), CamMath.height(camView, camHeight, dy, resources.displayMetrics.density)) }   // spread = closer, both fingers up = higher
        root.addView(drive, FrameLayout.LayoutParams(-1, -1))
        hud = Hud(this)
        root.addView(hud, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(24f), 0, 0) })
        fun button(label: String, gravity: Int, top: Float, end: Float, w: Float = 120f, onClick: () -> Unit) =
            M3Widgets.button(this, label, M3Widgets.ButtonKind.TONAL, onClick).also {
                root.addView(it, FrameLayout.LayoutParams(px(w), px(48f), gravity).apply { setMargins(0, px(top), px(end), 0) })
            }
        button(t("Reset", "Сброс"), Gravity.TOP or Gravity.END, 24f, 16f) { autopilotOn = false; showAutopilot(); Native.gameReset() }
        newLevelBtn = button(t("New level", "Новый уровень"), Gravity.TOP or Gravity.END, 24f, 150f) { autopilotOn = false; showAutopilot(); seed = System.nanoTime() and 0xFFFFFFL; Native.gameNewLevel(seed) }
        autopilotBtn = button(t("Autopilot", "Автопилот"), Gravity.TOP or Gravity.END, 24f, if (rallyIdx > 0) 150f else 284f, 168f) { setAutopilot(!autopilotOn) }
        autopilotBtn?.setOnLongClickListener { showAutopilotPanel(); true }   // long press: the autopilot settings
        // the camera view button under the status line: a tap cycles the views, a long press puts the current view back to its default distance and height
        camView = intent.getIntExtra("camview", Controls.camView(this)).coerceIn(0, CamMath.VIEWS - 1)   // debug/test: `camview`, `camdist`, `camheight` pick a view
        camDist = CamMath.clampDist(intent.getFloatExtra("camdist", Controls.camDist(this, camView))); camHeight = CamMath.clampHeight(camView, intent.getFloatExtra("camheight", Controls.camHeight(this, camView)))
        camBtn = M3Widgets.button(this, camLabel(), M3Widgets.ButtonKind.TONAL) { selectCameraView(CamMath.next(camView)) }.also {
            root.addView(it, FrameLayout.LayoutParams(px(150f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(16f), px(88f), 0, 0) })
            it.setOnLongClickListener { changeCamera(1f, 0f); true }
        }
        // the quick settings: a gear next to the View button opens a panel of the dust and effect options that applies at once, while driving
        M3Widgets.button(this, "⚙", M3Widgets.ButtonKind.TONAL) { showSettingsPanel() }.also {
            root.addView(it, FrameLayout.LayoutParams(px(56f), px(48f), Gravity.TOP or Gravity.START).apply { setMargins(px(174f), px(88f), 0, 0) })
        }
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
        intent.getFloatExtra("tod", -1f).let { if (it >= 0f) Native.gameTimeOfDay(it) }   // debug/test: time of day
        Native.gameStart(h.surface, w, hh, seed, profile, false)
        Native.gameQuality(Quality.index(this))
        intent.getIntExtra("dust", -1).let { if (it >= 0) Controls.setDust(this, it) }   // debug/test: the dust level (3 = fluid)
        Native.gameDust(Controls.dust(this))
        Native.gameAero(if (Controls.aero(this)) 1 else 0)
        intent.getIntExtra("blur", -1).let { if (it >= 0) Controls.setMotionBlur(this, it) }   // debug/test: motion blur level
        intent.getIntExtra("dustopts", -1).let { if (it >= 0) Controls.setDustOpts(this, it) }   // debug/test: the GPU dust options as bits
        Native.gameMotionBlur(Controls.motionBlur(this))
        Native.gameDustOpts(Controls.dustOpts(this))
        Native.gameSigns(Controls.signs(this))
        Native.gameCameraView(camView, camDist, camHeight)
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
        intent.getFloatExtra("steer", 0f).let { st -> if (st != 0f) handler.postDelayed({ carSteer = st; sendInput() }, 1500) }
        intent.getFloatExtra("throttle", 0f).let { th -> if (th != 0f) handler.postDelayed({ carThr = th; sendInput() }, 1500) }
        audio.start()
    }
    override fun onDestroy() { tts?.shutdown(); voicePlayer?.release(); super.onDestroy() }
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
        fun applyLive() { if (running) { Native.gameDust(Controls.dust(this)); Native.gameAero(if (Controls.aero(this)) 1 else 0); Native.gameDustOpts(Controls.dustOpts(this)); Native.gameMotionBlur(Controls.motionBlur(this)) }; refreshers.forEach { it() } }
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
        row(t("Dust", "Пыль"), { arrayOf(t("Off", "Выкл"), t("Normal", "Обычная"), t("Lots", "Много"), t("Fluid (test)", "Жидкая (опыт)"))[Controls.dust(this)] }) { Controls.setDust(this, DriveMath.cycle(Controls.dust(this), 4)) }
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

    private fun camLabel() = t("View: ", "Вид: ") + CamMath.name(camView, java.util.Locale.getDefault().language != "ru")

    /** Takes the distance (zoom) and height (tilt) of the current view, keeps them for next time and sends them to the game. */
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
        if (running) { Native.gameDust(Controls.dust(this)); Native.gameAero(if (Controls.aero(this)) 1 else 0); Native.gameMotionBlur(Controls.motionBlur(this)); Native.gameDustOpts(Controls.dustOpts(this)) }
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
