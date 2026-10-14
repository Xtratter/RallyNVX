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
        val a = floatArrayOf(4000f, 1f, 2f,  1f, 0f, 300f, 0f, 0f, 0f, 0f,  0f, 3000f, 750f, 2f, 9f, 0f, 1f)
        val i = GarageMath.parse(a)!!
        assertEquals(4000, i.money); assertEquals(1, i.selected); assertEquals(2, i.cars.size)
        assertTrue(i.cars[0].owned); assertFalse(i.cars[1].owned); assertEquals(3000, i.cars[1].price); assertEquals(750, i.cars[1].upgradeCost)
        assertArrayEquals(intArrayOf(2, 3, 0, 1), i.cars[1].levels)       // 9 clamps to the cap
        assertNull(GarageMath.parse(floatArrayOf(1f, 2f))); assertNull(GarageMath.parse(floatArrayOf(1f, 0f, 3f, 1f)))
        assertEquals(0, GarageMath.parse(floatArrayOf(5f, 7f, 1f, 1f, 0f, 300f, 0f, 0f, 0f, 0f))!!.selected)   // a wild index is clamped
    }
}
