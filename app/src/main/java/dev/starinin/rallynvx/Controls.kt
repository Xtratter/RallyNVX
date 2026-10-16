package dev.starinin.rallynvx

import android.app.Activity
import android.content.Context

/** Driving preferences kept in the app prefs (safe defaults when missing or out of range). */
object Controls {
    const val MODE_BUTTONS = 0
    const val MODE_WHEEL = 1
    const val MODE_TILT = 2
    const val MODES = 3
    private fun p(c: Context) = c.getSharedPreferences("prefs", Activity.MODE_PRIVATE)
    fun mode(c: Context) = p(c).getInt("steer_mode2", MODE_BUTTONS).coerceIn(0, MODES - 1)
    fun sens(c: Context) = p(c).getInt("steer_sens", 1).coerceIn(0, DriveMath.SENS.size - 1)
    fun pedalsLeft(c: Context) = p(c).getBoolean("pedals_left", false)
    fun volumeStep(c: Context) = p(c).getInt("volume_step", 3).coerceIn(0, DriveMath.VOLUME.size - 1)
    fun volume(c: Context) = DriveMath.VOLUME[volumeStep(c)]
    fun setMode(c: Context, v: Int) = p(c).edit().putInt("steer_mode2", v.coerceIn(0, MODES - 1)).apply()
    /** Pedals: false = full gas and full brake on touch (default), true = analog by touch height. */
    fun pedalsAnalog(c: Context) = p(c).getBoolean("pedals_analog", false)
    fun setPedalsAnalog(c: Context, v: Boolean) = p(c).edit().putBoolean("pedals_analog", v).apply()
    fun setSens(c: Context, v: Int) = p(c).edit().putInt("steer_sens", v.coerceIn(0, DriveMath.SENS.size - 1)).apply()
    fun setPedalsLeft(c: Context, v: Boolean) = p(c).edit().putBoolean("pedals_left", v).apply()
    fun setVolumeStep(c: Context, v: Int) = p(c).edit().putInt("volume_step", v.coerceIn(0, DriveMath.VOLUME.size - 1)).apply()
    /** Render mode: 0 raster, 1 hybrid; until the user picks one it follows the graphics quality (hybrid on High). */
    fun renderMode(c: Context): Int { val v = p(c).getInt("render_mode", -1); return if (v in 0..1) v else DriveMath.defaultRender(p(c).getInt("game_quality", 1)) }
    fun setRenderMode(c: Context, v: Int) = p(c).edit().putInt("render_mode", v.coerceIn(0, 1)).putInt("render_rev", renderRev(c) + 1).apply()
    fun renderRev(c: Context) = p(c).getInt("render_rev", 0)
    fun dust(c: Context): Int = p(c).getInt("dust", 1).coerceIn(0, 2)
    /** Signs along the stage (corner warnings, chevrons, boards). */
    fun signs(c: Context): Boolean = p(c).getBoolean("signs", true)
    fun setSigns(c: Context, v: Boolean) = p(c).edit().putBoolean("signs", v).apply()
    /** The chosen studio voice per language ("ru" / "en"): an id of assets/voices, or "system" for the speech synthesizer. */
    fun voice(c: Context, lang: String): String = p(c).getString("voice_$lang", null) ?: "system"
    fun setVoice(c: Context, lang: String, id: String) = p(c).edit().putString("voice_$lang", id).apply()
    /** The co-driver in rallies: 0 off, 1 icons, 2 icons and voice (the default). */
    fun paceMode(c: Context): Int = p(c).getInt("pace_mode", 2).coerceIn(0, 2)
    fun setPaceMode(c: Context, v: Int) = p(c).edit().putInt("pace_mode", v.coerceIn(0, 2)).apply()
    /** Autopilot style: 0 calm, 1 normal, 2 aggressive, 3 the player's own numbers. */
    const val AP_STYLES = 4
    fun autopilotStyle(c: Context): Int = p(c).getInt("autopilot_style", 1).coerceIn(0, AP_STYLES - 1)
    fun setAutopilotStyle(c: Context, v: Int) = p(c).edit().putInt("autopilot_style", v.coerceIn(0, AP_STYLES - 1)).apply()
    /** The own style's numbers as steps of these lists (cornering speed, braking, top speed, corner cutting, edge margin). */
    val AP_LAT = floatArrayOf(0.5f, 0.7f, 0.9f, 1.1f, 1.3f, 1.5f, 1.7f)
    val AP_BRAKE = floatArrayOf(0.5f, 0.65f, 0.8f, 0.95f, 1.1f)
    val AP_SPEED = floatArrayOf(0.5f, 0.6f, 0.7f, 0.8f, 0.9f, 1.0f)
    val AP_CUT = floatArrayOf(0.0f, 0.25f, 0.4f, 0.55f, 0.7f, 0.85f)
    val AP_EDGE = floatArrayOf(0.8f, 1.4f, 2.0f, 3.0f)
    private val AP_DEFAULT = mapOf("ap_lat" to 2, "ap_brake" to 2, "ap_speed" to 5, "ap_cut" to 3, "ap_slide" to 0, "ap_edge" to 1)
    fun apStep(c: Context, key: String, size: Int): Int = p(c).getInt(key, AP_DEFAULT[key] ?: 0).coerceIn(0, size - 1)
    fun setApStep(c: Context, key: String, v: Int) = p(c).edit().putInt(key, v).apply()
    /** Aerodynamics: the dust follows the airflow around the car (on the GPU); off = the simple CPU particles. */
    fun aero(c: Context): Boolean = p(c).getBoolean("aero", true)
    fun setAero(c: Context, v: Boolean) = p(c).edit().putBoolean("aero", v).apply()
    /** The camera view and, per view, the distance scale and the height offset the player chose. */
    fun camView(c: Context): Int = p(c).getInt("cam_view", 0).coerceIn(0, CamMath.VIEWS - 1)
    fun camDist(c: Context, view: Int): Float = CamMath.clampDist(p(c).getFloat("cam_dist_$view", 1f))
    fun camHeight(c: Context, view: Int): Float = CamMath.clampHeight(view, p(c).getFloat("cam_height_$view", 0f))
    fun setCam(c: Context, view: Int, dist: Float, height: Float) = p(c).edit().putInt("cam_view", view).putFloat("cam_dist_$view", dist).putFloat("cam_height_$view", height).apply()
    /** Motion blur (the sense of speed): 0 off (the default), 1 low, 2 high. */
    /** The weather the player asks for: 0 auto, 1 clear, 2 cloudy, 3 rain, 4 heavy rain, 5 fog. */
    fun weather(c: Context): Int = p(c).getInt("weather", 0).coerceIn(0, WeatherMode.COUNT - 1)
    fun setWeather(c: Context, v: Int) = p(c).edit().putInt("weather", v.coerceIn(0, WeatherMode.COUNT - 1)).apply()
    /** The manual transmission is chosen (the clutch pedal and the shift buttons are shown). */
    fun manual(c: Context): Boolean = p(c).getBoolean("manual_gearbox", false)
    fun setManual(c: Context, v: Boolean) = p(c).edit().putBoolean("manual_gearbox", v).apply()
    fun shadows(c: Context): Int = p(c).getInt("shadows", 0).coerceIn(0, 3)
    fun setShadows(c: Context, v: Int) = p(c).edit().putInt("shadows", v.coerceIn(0, 3)).apply()
    fun motionBlur(c: Context): Int = p(c).getInt("motion_blur", 0).coerceIn(0, 2)
    fun setMotionBlur(c: Context, v: Int) = p(c).edit().putInt("motion_blur", v.coerceIn(0, 2)).apply()
    /** The GPU dust options (bits: 1 particle pressure, 2 the car's exact shape, 4 collisions with the world, 8 the settled layer); all but the pressure by default. */
    const val DUST_OPTS_DEFAULT = 14
    fun dustOpts(c: Context): Int = p(c).getInt("dust_opts", DUST_OPTS_DEFAULT) and 15
    fun setDustOpts(c: Context, v: Int) = p(c).edit().putInt("dust_opts", v and 15).apply()
    fun dustOpt(c: Context, bit: Int): Boolean = dustOpts(c) and bit != 0
    fun toggleDustOpt(c: Context, bit: Int) = setDustOpts(c, dustOpts(c) xor bit)
    /** Inversion of the camera drag by a finger: 0 none, 1 horizontal, 2 vertical, 3 both. */
    fun camInvert(c: Context): Int = p(c).getInt("cam_invert", 0).coerceIn(0, CamMath.INVERT_MODES - 1)
    fun setCamInvert(c: Context, v: Int) = p(c).edit().putInt("cam_invert", v.coerceIn(0, CamMath.INVERT_MODES - 1)).apply()
    fun setDust(c: Context, v: Int) = p(c).edit().putInt("dust", v.coerceIn(0, 3)).apply()
    fun autoFallback(c: Context) = p(c).getBoolean("auto_fallback", false)
    fun setAutoFallback(c: Context, v: Boolean) = p(c).edit().putBoolean("auto_fallback", v).apply()
    fun hintSeen(c: Context, mode: Int) = p(c).getBoolean("hint_$mode", false)
    fun setHintSeen(c: Context, mode: Int, v: Boolean) = p(c).edit().putBoolean("hint_$mode", v).apply()
    fun resetHints(c: Context) = p(c).edit().remove("hint_0").remove("hint_1").remove("hint_2").apply()
    /** Set by the settings screen; the game re-zeroes the tilt when it sees it (and clears it). */
    fun wantsCalibration(c: Context) = p(c).getBoolean("recalibrate", false)
    fun setWantsCalibration(c: Context, v: Boolean) = p(c).edit().putBoolean("recalibrate", v).apply()
}
