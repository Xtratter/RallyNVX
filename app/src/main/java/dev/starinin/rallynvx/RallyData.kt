package dev.starinin.rallynvx

import android.app.Activity
import android.content.Context

/** The rallies (a fixed seed each, so the records mean something), their stage names, the time format and the records. */
object RallyData {
    val SEEDS = longArrayOf(21, 3, 8, 5)
    private val NAMES = arrayOf("Foothills" to "Предгорья", "Lakeside" to "Озёрный край", "Highland" to "Нагорье", "Valley" to "Долина")
    private val STAGES = arrayOf("Special stage" to "Спецучасток", "City super-special" to "Городская суперспециалка")
    const val STAGE_COUNT = 2

    fun name(rally: Int) = NAMES[rally.coerceIn(0, NAMES.size - 1)].let { t(it.first, it.second) }
    fun stageName(stage: Int) = STAGES[stage.coerceIn(0, STAGES.size - 1)].let { t(it.first, it.second) }

    /** Seconds as m:ss.t (tenths cut, not rounded, like a stopwatch); negative or broken values show 0:00.0. */
    fun time(sec: Float): String {
        val s = if (sec.isFinite() && sec > 0f) sec else 0f
        val tenths = (s * 10f).toInt()
        return "%d:%02d.%d".format(tenths / 600, (tenths / 10) % 60, tenths % 10)
    }

    private fun p(c: Context) = c.getSharedPreferences("prefs", Activity.MODE_PRIVATE)
    private fun key(rally: Int, stage: Int) = if (stage < 0) "rec_${rally}_total" else "rec_${rally}_$stage"

    /** The best total of a stage (`stage` -1: of the whole rally) in seconds, 0 when there is none yet. */
    fun record(c: Context, rally: Int, stage: Int): Float = p(c).getFloat(key(rally, stage), 0f)

    /** Saves `total` when it beats the record (or there is none); true when it is a new record. */
    fun saveIfBetter(c: Context, rally: Int, stage: Int, total: Float): Boolean {
        if (!total.isFinite() || total <= 0f) return false
        val old = record(c, rally, stage)
        if (old > 0f && total >= old) return false
        p(c).edit().putFloat(key(rally, stage), total).apply()
        return true
    }
}
