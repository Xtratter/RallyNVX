package dev.starinin.rallynvx

/** The quick time-of-day switch in the drive: Auto lets the level's own clock run, the others freeze the sky at a phase (0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk). */
object TimeOfDay {
    /** Native phase of each mode; negative = automatic (the level's own clock). */
    val PHASES = floatArrayOf(-1f, 0.5f, 0.74f, 0.0f)
    val EN = arrayOf("Auto", "Day", "Evening", "Night")
    val RU = arrayOf("Авто", "День", "Вечер", "Ночь")
    const val COUNT = 4

    fun next(mode: Int) = (mode + 1).mod(COUNT)
    fun phase(mode: Int) = PHASES[mode.mod(COUNT)]
    fun name(mode: Int, en: Boolean) = (if (en) EN else RU)[mode.mod(COUNT)]
}
