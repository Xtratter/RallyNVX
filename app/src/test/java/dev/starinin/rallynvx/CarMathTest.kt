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
    @Test fun differentialPartsHaveNamesRangesAndTexts() {
        assertEquals(12, CarMath.PARTS)
        assertEquals("Front diff lock", CarMath.partName(7, true)); assertEquals("Задний дифференциал", CarMath.partName(8, false))
        assertEquals(0f, CarMath.lo(9), 0f); assertEquals(2f, CarMath.hi(9), 0f); assertEquals(0.6f, CarMath.lo(10), 1e-6f); assertEquals(1.4f, CarMath.hi(10), 1e-6f)
        assertEquals("50/50", CarMath.valueText(10, 1f)); assertEquals("70/30", CarMath.valueText(10, 1.4f)); assertEquals("30/70", CarMath.valueText(10, 0.6f))
        assertEquals("150 %", CarMath.valueText(7, 1.5f))
    }
    @Test fun presetsAreInsideTheRangesAndWrap() {
        for (p in 0 until CarMath.PRESET_COUNT) for (k in 0 until 4) { val v = CarMath.PRESETS[p][k]; assertEquals(v, CarMath.clamp(7 + k, v), 1e-6f) }
        assertEquals("Gravel", CarMath.presetName(1, true)); assertEquals("Снег", CarMath.presetName(2, false)); assertEquals("Tarmac", CarMath.presetName(99, true))
    }
    @Test fun launchControlAndTheBrakeBias() {
        assertEquals(8, CarMath.AIDS); assertEquals(0xFF, CarMath.ALL_AIDS)
        assertEquals("Launch control", CarMath.aidName(7, true)); assertEquals("Launch control (старт)", CarMath.aidName(7, false))
        assertEquals(12, CarMath.PARTS)
        assertEquals("Brake bias (front/rear)", CarMath.partName(11, true)); assertEquals("Баланс тормозов (перед/зад)", CarMath.partName(11, false))
        assertEquals(0.7f, CarMath.lo(11), 1e-6f); assertEquals(1.25f, CarMath.hi(11), 1e-6f)
        assertEquals("62/38", CarMath.valueText(11, 1f)); assertEquals("68/32", CarMath.valueText(11, 1.1f)); assertEquals("43/57", CarMath.valueText(11, 0.7f))
    }
}
