package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class GarageMathTest {
    @Test fun pipsShowTheLevelAndClamp() {
        assertEquals("○○○", GarageMath.pips(0)); assertEquals("●○○", GarageMath.pips(1)); assertEquals("●●●", GarageMath.pips(3))
        assertEquals("●●●", GarageMath.pips(9)); assertEquals("○○○", GarageMath.pips(-4))
    }
    @Test fun priceLabel() { assertEquals("300 $", GarageMath.price(300)) }
    @Test fun stepWrapsBothWays() {
        assertEquals(1, GarageMath.step(0, 1, 2)); assertEquals(0, GarageMath.step(1, 1, 2)); assertEquals(1, GarageMath.step(0, -1, 2))
        assertEquals(0, GarageMath.step(0, 1, 0)); assertEquals(2, GarageMath.step(0, -1, 3))
    }
    @Test fun upgradePricesAreOneTwoFourTimesTheBaseAndNullAtTheCap() {
        assertEquals(300, GarageMath.upgradePrice(300, 0)); assertEquals(600, GarageMath.upgradePrice(300, 1)); assertEquals(1200, GarageMath.upgradePrice(300, 2))
        assertNull(GarageMath.upgradePrice(300, 3)); assertNull(GarageMath.upgradePrice(300, -1))
    }
    @Test fun parsesTheInfoArrayAndRejectsShortOnes() {
        fun car(owned: Float, price: Float, cost: Float, lv: FloatArray, angle: Float, kg: Float, pct: Float, top: Float) = floatArrayOf(owned, price, cost) + lv + floatArrayOf(angle, kg, pct, top)
        val a = floatArrayOf(4000f, 1f, 2f) + car(1f, 0f, 300f, FloatArray(7), 1f, 12f, 0f, 150f) + car(0f, 3000f, 750f, floatArrayOf(2f, 9f, 0f, 1f, 3f, 2f, 0f), 2f, 140.5f, 38f, 142f)
        val i = GarageMath.parse(a)!!
        assertEquals(4000, i.money); assertEquals(1, i.selected); assertEquals(2, i.cars.size)
        assertTrue(i.cars[0].owned); assertFalse(i.cars[1].owned); assertEquals(3000, i.cars[1].price); assertEquals(750, i.cars[1].upgradeCost)
        assertArrayEquals(intArrayOf(2, 3, 0, 1, 3, 2, 0), i.cars[1].levels)       // 9 clamps to the cap
        assertEquals(2, i.cars[1].wingAngle); assertEquals(140.5f, i.cars[1].downforceKg, 0.01f); assertEquals(38f, i.cars[1].dragPct, 0.01f); assertEquals(142f, i.cars[1].topKmh, 0.01f)
        assertNull(GarageMath.parse(floatArrayOf(1f, 2f))); assertNull(GarageMath.parse(floatArrayOf(1f, 0f, 3f, 1f)))
        assertEquals(0, GarageMath.parse(floatArrayOf(5f, 7f, 1f) + car(1f, 0f, 300f, FloatArray(7), 1f, 0f, 0f, 0f))!!.selected)   // a wild index is clamped
    }
    @Test fun wingAngleLabels() {
        assertEquals("Low", GarageMath.wingAngleLabel(0, true)); assertEquals("Средний", GarageMath.wingAngleLabel(1, false)); assertEquals("High", GarageMath.wingAngleLabel(7, true))
    }
}
