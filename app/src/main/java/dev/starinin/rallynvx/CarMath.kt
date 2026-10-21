package dev.starinin.rallynvx

/** The driver aids and the tuning of the car's parts (the game's "Car" menu); the bits and the ranges are those of `rust/src/game/assists.rs`. */
object CarMath {
    const val ALL_AIDS = 0xFF
    /** The aids in the order of the bits: anti-lag, ABS, traction control, the blip, the flat shift, the limiter, the blow-off valve, launch control. */
    fun aidName(bit: Int, en: Boolean): String = when (bit) {
        0 -> if (en) "Anti-lag" else "Антилаг"
        1 -> if (en) "ABS" else "АБС"
        2 -> if (en) "Traction control" else "Антибуксовка (TC)"
        3 -> if (en) "Downshift blip" else "Перегазовка при даун-шифте"
        4 -> if (en) "Flat shift (spark cut)" else "Отсечка при переключении"
        5 -> if (en) "Rev limiter" else "Ограничитель оборотов"
        6 -> if (en) "Blow-off valve" else "Блоуофф"
        else -> if (en) "Launch control" else "Launch control (старт)"
    }
    const val AIDS = 8
    fun aidOn(mask: Int, bit: Int) = mask and (1 shl bit) != 0
    fun toggle(mask: Int, bit: Int) = (mask xor (1 shl bit)) and ALL_AIDS

    const val PARTS = 13
    /** The tuned parts: the engine, the turbo, the brakes, the tyres, the suspension, the downforce, the gearing. */
    fun partName(i: Int, en: Boolean): String = when (i) {
        0 -> if (en) "Engine power" else "Мощность двигателя"
        1 -> if (en) "Turbo boost" else "Наддув турбины"
        2 -> if (en) "Brakes" else "Тормоза"
        3 -> if (en) "Tyre grip" else "Сцепление шин"
        4 -> if (en) "Suspension" else "Подвеска"
        5 -> if (en) "Downforce" else "Прижимная сила"
        7 -> if (en) "Front diff lock" else "Передний дифференциал"
        8 -> if (en) "Rear diff lock" else "Задний дифференциал"
        9 -> if (en) "Centre diff lock" else "Центральный дифференциал"
        10 -> if (en) "Centre split (front/rear)" else "Распределение (перед/зад)"
        11 -> if (en) "Brake bias (front/rear)" else "Баланс тормозов (перед/зад)"
        12 -> if (en) "Tyre pressure (cold)" else "Давление в шинах (холодных)"
        else -> if (en) "Final drive" else "Главная пара"
    }
    private val LO = floatArrayOf(0.3f, 0f, 0.3f, 0.6f, 0.5f, 0f, 0.7f, 0f, 0f, 0f, 0.6f, 0.7f, 0.8f)
    private val HI = floatArrayOf(2f, 2f, 2.5f, 1.5f, 1.8f, 3f, 1.3f, 2f, 2f, 2f, 1.4f, 1.25f, 1.25f)
    fun lo(i: Int) = LO[i.coerceIn(0, PARTS - 1)]
    fun hi(i: Int) = HI[i.coerceIn(0, PARTS - 1)]
    fun clamp(i: Int, v: Float) = if (v.isFinite()) v.coerceIn(lo(i), hi(i)) else 1f
    /** One step (10 %) up or down, kept to a tenth and inside the range. */
    fun step(i: Int, v: Float, dir: Int): Float = clamp(i, Math.round((v + 0.1f * dir) * 10f) / 10f)
    fun percent(v: Float) = Math.round(v * 100f).toString() + " %"
    /** The text of a part's value: a percentage of the built value, and for the centre split (part 10, 1 = 50/50) the share to the front and to the rear. */
    fun valueText(i: Int, v: Float): String = when (i) { 10 -> { val f = Math.round(50f * v); "$f/${100 - f}" }; 11 -> { val f = Math.round(62f * v); "$f/${100 - f}" }; else -> percent(v) }
    /** The tyres in two lines: the temperature (degrees C) and the wear (percent) of FL, FR, RL, RR; garbage reads as zero. */
    fun tyreLine(temp: FloatArray, wear: FloatArray, en: Boolean): String {
        val names = if (en) arrayOf("FL", "FR", "RL", "RR") else arrayOf("ПЛ", "ПП", "ЗЛ", "ЗП")
        fun one(k: Int): String {
            val t = temp.getOrElse(k) { 0f }.let { if (it.isFinite()) Math.round(it) else 0 }
            val w = wear.getOrElse(k) { 0f }.let { if (it.isFinite()) Math.round(it.coerceIn(0f, 1f) * 100f) else 0 }
            return "${names[k]} ${t}°C ${w}%"
        }
        return one(0) + "  " + one(1) + "\n" + one(2) + "  " + one(3)
    }
    /** Differential presets for the surface (front lock, rear lock, centre lock, centre split): tarmac turns the car in with a looser centre and a rear bias, gravel is as built, snow locks the centre. */
    const val PRESET_COUNT = 3
    val PRESETS = arrayOf(floatArrayOf(0.8f, 1.2f, 0.7f, 0.9f), floatArrayOf(1f, 1f, 1f, 1f), floatArrayOf(0.8f, 0.8f, 1.6f, 1f))
    fun presetName(i: Int, en: Boolean): String = when (i) { 1 -> if (en) "Gravel" else "Гравий"; 2 -> if (en) "Snow" else "Снег"; else -> if (en) "Tarmac" else "Асфальт" }
}
