package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class CarMathTest {
    @Test fun aidsToggleOneBitEach() {
        var m = CarMath.ALL_AIDS
        for (b in 0 until CarMath.AIDS) { assertTrue(CarMath.aidOn(m, b)); m = CarMath.toggle(m, b); assertFalse(CarMath.aidOn(m, b)); m = CarMath.toggle(m, b); assertEquals(CarMath.ALL_AIDS, m) }
        assertEquals(0x7F, CarMath.toggle(0x7F, 9) and 0x7F)
    }
    @Test fun stepsStayInsideTheRangeAndOnTenths() {
        assertEquals(1.1f, CarMath.step(0, 1f, 1), 1e-6f); assertEquals(0.9f, CarMath.step(0, 1f, -1), 1e-6f)
        assertEquals(2f, CarMath.step(0, 2f, 1), 0f); assertEquals(0.3f, CarMath.step(0, 0.3f, -1), 1e-6f)
        assertEquals(0f, CarMath.step(1, 0f, -1), 0f); assertEquals(1.3f, CarMath.step(6, 1.3f, 1), 1e-6f)
        assertEquals(1f, CarMath.clamp(2, Float.NaN), 0f)
        var v = 1f; repeat(40) { v = CarMath.step(5, v, 1) }; assertEquals(3f, v, 1e-6f)
    }
    @Test fun labels() { assertEquals("110 %", CarMath.percent(1.1f)); assertEquals("ABS", CarMath.aidName(1, true)); assertEquals("Тормоза", CarMath.partName(2, false)) }
}
