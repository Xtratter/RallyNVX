package dev.starinin.raytrace

import android.app.Activity
import android.content.Context

/** Driving preferences kept in the app prefs (safe defaults when missing or out of range). */
object Controls {
    const val MODE_WHEEL = 0
    const val MODE_TILT = 1
    private fun p(c: Context) = c.getSharedPreferences("prefs", Activity.MODE_PRIVATE)
    fun mode(c: Context) = p(c).getInt("steer_mode", MODE_WHEEL).coerceIn(0, 1)
    fun sens(c: Context) = p(c).getInt("steer_sens", 1).coerceIn(0, DriveMath.SENS.size - 1)
    fun pedalsLeft(c: Context) = p(c).getBoolean("pedals_left", false)
    fun volumeStep(c: Context) = p(c).getInt("volume_step", 3).coerceIn(0, DriveMath.VOLUME.size - 1)
    fun volume(c: Context) = DriveMath.VOLUME[volumeStep(c)]
    fun setMode(c: Context, v: Int) = p(c).edit().putInt("steer_mode", v.coerceIn(0, 1)).apply()
    fun setSens(c: Context, v: Int) = p(c).edit().putInt("steer_sens", v.coerceIn(0, DriveMath.SENS.size - 1)).apply()
    fun setPedalsLeft(c: Context, v: Boolean) = p(c).edit().putBoolean("pedals_left", v).apply()
    fun setVolumeStep(c: Context, v: Int) = p(c).edit().putInt("volume_step", v.coerceIn(0, DriveMath.VOLUME.size - 1)).apply()
    fun hintSeen(c: Context, mode: Int) = p(c).getBoolean("hint_$mode", false)
    fun setHintSeen(c: Context, mode: Int, v: Boolean) = p(c).edit().putBoolean("hint_$mode", v).apply()
    fun resetHints(c: Context) = p(c).edit().remove("hint_0").remove("hint_1").apply()
    /** Set by the settings screen; the game re-zeroes the tilt when it sees it (and clears it). */
    fun wantsCalibration(c: Context) = p(c).getBoolean("recalibrate", false)
    fun setWantsCalibration(c: Context, v: Boolean) = p(c).edit().putBoolean("recalibrate", v).apply()
}
