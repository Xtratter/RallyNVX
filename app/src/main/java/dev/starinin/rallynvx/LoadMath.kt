package dev.starinin.rallynvx

/** The start-up progress the game thread reports (`Native.gameLoadProgress`: stage id shl 8 or percent). */
object LoadMath {
    const val DONE = 100
    fun percent(packed: Int): Int = (packed and 0xFF).coerceIn(0, 100)
    fun stage(packed: Int): Int = packed shr 8
    fun done(packed: Int): Boolean = percent(packed) >= DONE

    fun label(packed: Int, en: Boolean): String = when (stage(packed)) {
        1 -> if (en) "Starting the graphics" else "Запуск графики"
        2 -> if (en) "Loading textures" else "Загрузка текстур"
        3 -> if (en) "Compiling shaders" else "Компиляция шейдеров"
        4 -> if (en) "Building the world" else "Создание мира"
        5 -> if (en) "Loading the car" else "Загрузка машины"
        6 -> if (en) "Almost ready" else "Почти готово"
        else -> if (en) "Loading" else "Загрузка"
    }

    /** The shown fraction eases towards the reported one (never back), so the bar moves between the reports. */
    fun ease(shown: Float, target: Float, dt: Float): Float = if (target <= shown) shown else (shown + (target - shown) * (1f - Math.exp(-6.0 * dt).toFloat())).coerceAtMost(target)
}
