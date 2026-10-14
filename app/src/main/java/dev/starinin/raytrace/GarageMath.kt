package dev.starinin.raytrace

/** Pure helpers of the garage screen (no Android): upgrade pips, price labels, car index stepping, info parsing. */
object GarageMath {
    const val CAR_FIELDS = 7           // owned, price, upgrade_cost, engine, tyres, suspension, brakes
    const val MAX_LEVEL = 3

    fun pips(level: Int): String = "●".repeat(level.coerceIn(0, MAX_LEVEL)) + "○".repeat(MAX_LEVEL - level.coerceIn(0, MAX_LEVEL))

    fun price(value: Int): String = "$value $"

    /** The next car index when stepping by [delta] (-1 or 1), wrapping around. */
    fun step(index: Int, delta: Int, count: Int): Int = if (count <= 0) 0 else ((index + delta) % count + count) % count

    /** Price of the next level of an upgrade at [level] for a car with [upgradeCost] (1x, 2x, 4x), or null at the cap. */
    fun upgradePrice(upgradeCost: Int, level: Int): Int? = if (level >= MAX_LEVEL || level < 0) null else upgradeCost * intArrayOf(1, 2, 4)[level]

    class Car(val owned: Boolean, val price: Int, val upgradeCost: Int, val levels: IntArray)
    class Info(val money: Int, val selected: Int, val cars: List<Car>)

    /** Parses `Native.garageInfo`; null when the array is too short or inconsistent. */
    fun parse(a: FloatArray): Info? {
        if (a.size < 3) return null
        val n = a[2].toInt()
        if (n < 0 || a.size < 3 + n * CAR_FIELDS) return null
        val cars = (0 until n).map { i ->
            val o = 3 + i * CAR_FIELDS
            Car(a[o] > 0.5f, a[o + 1].toInt(), a[o + 2].toInt(), IntArray(4) { a[o + 3 + it].toInt().coerceIn(0, MAX_LEVEL) })
        }
        return Info(a[0].toInt(), a[1].toInt().coerceIn(0, maxOf(n - 1, 0)), cars)
    }
}
