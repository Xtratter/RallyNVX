package dev.starinin.rallynvx

/** Pure helpers of the garage screen (no Android): upgrade pips, price labels, car index stepping, info parsing. */
object GarageMath {
    const val CAR_FIELDS = 15          // owned, price, upgrade_cost, engine, tyres, suspension, brakes, splitter, wing, underbody, wing angle, wheel set, downforce kg, drag %, top km/h
    const val UPGRADES = 7
    const val MAX_LEVEL = 3

    fun pips(level: Int): String = "●".repeat(level.coerceIn(0, MAX_LEVEL)) + "○".repeat(MAX_LEVEL - level.coerceIn(0, MAX_LEVEL))

    fun price(value: Int): String = "$value $"

    /** The next car index when stepping by [delta] (-1 or 1), wrapping around. */
    fun step(index: Int, delta: Int, count: Int): Int = if (count <= 0) 0 else ((index + delta) % count + count) % count

    /** Price of the next level of an upgrade at [level] for a car with [upgradeCost] (1x, 2x, 4x), or null at the cap. */
    fun upgradePrice(upgradeCost: Int, level: Int): Int? = if (level >= MAX_LEVEL || level < 0) null else upgradeCost * intArrayOf(1, 2, 4)[level]

    class Car(val owned: Boolean, val price: Int, val upgradeCost: Int, val levels: IntArray, val wingAngle: Int, val wheelSet: Int, val downforceKg: Float, val dragPct: Float, val topKmh: Float)
    class Info(val money: Int, val selected: Int, val cars: List<Car>)

    /** The label of a wing angle (0 low drag, 1 mid, 2 high downforce). */
    fun wingAngleLabel(angle: Int, en: Boolean): String = arrayOf(if (en) "Low" else "Низкий", if (en) "Mid" else "Средний", if (en) "High" else "Высокий")[angle.coerceIn(0, 2)]

    /** The label of a wheel set (0 tarmac, 1 gravel, 2 snow). */
    fun wheelSetLabel(set: Int, en: Boolean): String = arrayOf(if (en) "Tarmac" else "Асфальт", if (en) "Gravel" else "Гравий", if (en) "Snow" else "Снег")[set.coerceIn(0, 2)]

    /** Parses `Native.garageInfo`; null when the array is too short or inconsistent. */
    fun parse(a: FloatArray): Info? {
        if (a.size < 3) return null
        val n = a[2].toInt()
        if (n < 0 || a.size < 3 + n * CAR_FIELDS) return null
        val cars = (0 until n).map { i ->
            val o = 3 + i * CAR_FIELDS
            Car(a[o] > 0.5f, a[o + 1].toInt(), a[o + 2].toInt(), IntArray(UPGRADES) { a[o + 3 + it].toInt().coerceIn(0, MAX_LEVEL) }, a[o + 10].toInt().coerceIn(0, 2), a[o + 11].toInt().coerceIn(0, 2), a[o + 12], a[o + 13], a[o + 14])
        }
        return Info(a[0].toInt(), a[1].toInt().coerceIn(0, maxOf(n - 1, 0)), cars)
    }
}
