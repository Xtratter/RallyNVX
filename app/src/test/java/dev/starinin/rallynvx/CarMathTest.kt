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
        assertEquals(19, CarMath.PARTS)
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
        assertEquals(19, CarMath.PARTS)
        assertEquals("Brake bias (front/rear)", CarMath.partName(11, true)); assertEquals("Баланс тормозов (перед/зад)", CarMath.partName(11, false))
        assertEquals(0.7f, CarMath.lo(11), 1e-6f); assertEquals(1.25f, CarMath.hi(11), 1e-6f)
        assertEquals("62/38", CarMath.valueText(11, 1f)); assertEquals("68/32", CarMath.valueText(11, 1.1f)); assertEquals("43/57", CarMath.valueText(11, 0.7f))
    }
    @Test fun tyrePressurePartAndTheTyreLine() {
        assertEquals("Tyre pressure (cold)", CarMath.partName(12, true)); assertEquals("Давление в шинах (холодных)", CarMath.partName(12, false))
        assertEquals(0.8f, CarMath.lo(12), 1e-6f); assertEquals(1.25f, CarMath.hi(12), 1e-6f); assertEquals("110 %", CarMath.valueText(12, 1.1f))
        assertEquals("FL 85°C 3%  FR 90°C 50%\nRL 70°C 0%  RR 60°C 100%", CarMath.tyreLine(floatArrayOf(85.4f, 90f, 70f, 60f), floatArrayOf(0.03f, 0.5f, 0f, 1.2f), true))
        assertEquals("ПЛ -5°C 0%  ПП 0°C 0%\nЗЛ 0°C 0%  ЗП 0°C 0%", CarMath.tyreLine(floatArrayOf(-4.6f, Float.NaN, 0f, 0f), floatArrayOf(0f, Float.NaN, -1f, 0f), false))
    }
    @Test fun suspensionPartsHaveNamesRangesAndDegrees() {
        assertEquals("Front anti-roll bar", CarMath.partName(13, true)); assertEquals("Задний стабилизатор", CarMath.partName(14, false))
        assertEquals("Front camber", CarMath.partName(15, true)); assertEquals("Схождение сзади", CarMath.partName(18, false))
        for (i in 13..18) { assertEquals(0f, CarMath.lo(i), 0f); assertEquals(2f, CarMath.hi(i), 0f) }
        assertEquals("100 %", CarMath.valueText(13, 1f))
        assertEquals("-1.5°", CarMath.valueText(15, 1f)); assertEquals("-3.0°", CarMath.valueText(15, 2f)); assertEquals("-1.0°", CarMath.valueText(16, 1f)); assertEquals("0.0°", CarMath.valueText(15, 0f))
        assertEquals("0.0°", CarMath.valueText(17, 1f)); assertEquals("+0.4°", CarMath.valueText(17, 2f)); assertEquals("-0.1°", CarMath.valueText(18, 1f)); assertEquals("-0.5°", CarMath.valueText(18, 0f))
    }
    @Test fun suspensionPresetsAreInsideTheRanges() {
        assertEquals(4, CarMath.CHASSIS_PRESET_COUNT)
        for (p in 0 until CarMath.CHASSIS_PRESET_COUNT) for (k in 0 until 6) { val v = CarMath.CHASSIS_PRESETS[p][k]; assertEquals(v, CarMath.clamp(13 + k, v), 1e-6f) }
        assertEquals("Standard", CarMath.chassisPresetName(0, true)); assertEquals("Асфальт", CarMath.chassisPresetName(1, false)); assertEquals("Standard", CarMath.chassisPresetName(99, true))
        assertTrue(CarMath.CHASSIS_PRESETS[0].all { it == 1f })
    }
    @Test fun heatAndDamageLines() {
        assertEquals("Coolant 86°C  Clutch 12°C", CarMath.heatLine(86.4f, 11.6f, true)); assertEquals("Охлаждающая жидкость 130°C  Сцепление 0°C", CarMath.heatLine(130f, Float.NaN, false))
        assertEquals("Damage: body 0%  engine 25%  springs 10 20 30 100%", CarMath.damageLine(0f, 0.25f, floatArrayOf(0.1f, 0.2f, 0.3f, 1.7f), true))
        assertEquals("Повреждения: кузов 50%  двигатель 0%  пружины 0 0 0 0%", CarMath.damageLine(0.5f, Float.NaN, floatArrayOf(0f, 0f, 0f, -1f), false))
        assertTrue(CarMath.coolantHot(112f) && !CarMath.coolantHot(100f) && CarMath.clutchHot(300f) && !CarMath.clutchHot(120f))
    }
}
