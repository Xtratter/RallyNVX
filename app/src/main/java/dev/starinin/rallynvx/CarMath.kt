package dev.starinin.rallynvx

/** The driver aids and the tuning of the car's parts (the game's "Car" menu); the bits and the ranges are those of `rust/src/game/assists.rs`. */
object CarMath {
    const val ALL_AIDS = 0x7F
    /** The aids in the order of the bits: anti-lag, ABS, traction control, the blip, the flat shift, the limiter, the blow-off valve. */
    fun aidName(bit: Int, en: Boolean): String = when (bit) {
        0 -> if (en) "Anti-lag" else "Антилаг"
        1 -> if (en) "ABS" else "АБС"
        2 -> if (en) "Traction control" else "Антибуксовка (TC)"
        3 -> if (en) "Downshift blip" else "Перегазовка при даун-шифте"
        4 -> if (en) "Flat shift (spark cut)" else "Отсечка при переключении"
        5 -> if (en) "Rev limiter" else "Ограничитель оборотов"
        else -> if (en) "Blow-off valve" else "Блоуофф"
    }
    const val AIDS = 7
    fun aidOn(mask: Int, bit: Int) = mask and (1 shl bit) != 0
    fun toggle(mask: Int, bit: Int) = (mask xor (1 shl bit)) and ALL_AIDS

    const val PARTS = 7
    /** The tuned parts: the engine, the turbo, the brakes, the tyres, the suspension, the downforce, the gearing. */
    fun partName(i: Int, en: Boolean): String = when (i) {
        0 -> if (en) "Engine power" else "Мощность двигателя"
        1 -> if (en) "Turbo boost" else "Наддув турбины"
        2 -> if (en) "Brakes" else "Тормоза"
        3 -> if (en) "Tyre grip" else "Сцепление шин"
        4 -> if (en) "Suspension" else "Подвеска"
        5 -> if (en) "Downforce" else "Прижимная сила"
        else -> if (en) "Final drive" else "Главная пара"
    }
    private val LO = floatArrayOf(0.3f, 0f, 0.3f, 0.6f, 0.5f, 0f, 0.7f)
    private val HI = floatArrayOf(2f, 2f, 2.5f, 1.5f, 1.8f, 3f, 1.3f)
    fun lo(i: Int) = LO[i.coerceIn(0, PARTS - 1)]
    fun hi(i: Int) = HI[i.coerceIn(0, PARTS - 1)]
    fun clamp(i: Int, v: Float) = if (v.isFinite()) v.coerceIn(lo(i), hi(i)) else 1f
    /** One step (10 %) up or down, kept to a tenth and inside the range. */
    fun step(i: Int, v: Float, dir: Int): Float = clamp(i, Math.round((v + 0.1f * dir) * 10f) / 10f)
    fun percent(v: Float) = Math.round(v * 100f).toString() + " %"
}
