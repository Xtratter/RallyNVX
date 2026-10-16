package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class CamMathTest {
    @Test fun viewsCycleAndHaveNamesInBothLanguages() {
        var v = 0
        val seen = mutableSetOf<Int>()
        repeat(CamMath.VIEWS) { seen += v; v = CamMath.next(v) }
        assertEquals(CamMath.VIEWS, seen.size); assertEquals(0, v)
        assertEquals("Behind", CamMath.name(0, true)); assertEquals("Капот", CamMath.name(1, false))
        assertEquals("Free", CamMath.name(99, true)); assertEquals("Свободная", CamMath.name(6, false)); assertEquals("Cockpit", CamMath.name(5, true)); assertEquals("Салон", CamMath.name(5, false)); assertEquals("Orbit", CamMath.name(4, true)); assertEquals("Сзади", CamMath.name(-4, false))
        assertEquals(0, CamMath.next(-3).minus(1))   // a wild view index is clamped before cycling
    }
    @Test fun theFreeCameraFliesForwardOnASpreadUpOnAnUpwardDragAndStaysBounded() {
        assertTrue(CamMath.free(6) && !CamMath.free(4) && !CamMath.rigid(6))
        assertTrue(CamMath.flyForward(1.3f) > 0f && CamMath.flyForward(0.7f) < 0f && CamMath.flyForward(1f) == 0f)
        assertEquals(30f, CamMath.flyForward(1e9f), 0f); assertEquals(-30f, CamMath.flyForward(1e-9f), 0f)
        assertEquals(0f, CamMath.flyForward(Float.NaN), 0f); assertEquals(0f, CamMath.flyForward(-1f), 0f)
        assertTrue(CamMath.flyUp(-40f, 2.6f) > 0f && CamMath.flyUp(40f, 2.6f) < 0f && CamMath.flyUp(0f, 2.6f) == 0f)
        assertEquals(0f, CamMath.flyUp(Float.NaN, 2.6f), 0f); assertEquals(30f, CamMath.flyUp(-1e9f, 2.6f), 0f)
    }
    @Test fun distanceAndHeightAreClampedAndGarbageBecomesTheDefault() {
        assertEquals(CamMath.DIST_MAX, CamMath.clampDist(9f), 0f); assertEquals(CamMath.DIST_MIN, CamMath.clampDist(0.01f), 0f)
        assertEquals(1f, CamMath.clampDist(Float.NaN), 0f); assertEquals(1f, CamMath.clampDist(Float.POSITIVE_INFINITY), 0f)
        assertEquals(6f, CamMath.clampHeight(0, 99f), 0f); assertEquals(-3f, CamMath.clampHeight(3, -99f), 0f)
        assertEquals(1f, CamMath.clampHeight(1, 99f), 0f); assertEquals(-1f, CamMath.clampHeight(2, -99f), 0f)
        assertEquals(0f, CamMath.clampHeight(0, Float.NaN), 0f)
    }
    @Test fun spreadingFingersBringsTheOutsideViewsCloserAndZoomsTheRigidOnes() {
        assertTrue(CamMath.pinchFor(0, 1f, 1.5f) < 1f)    // apart: closer
        assertTrue(CamMath.pinchFor(0, 1f, 0.6f) > 1f)    // together: further
        assertTrue(CamMath.pinchFor(1, 1f, 1.5f) > 1f)    // bonnet: apart zooms in (a bigger scale is a narrower field)
        assertTrue(CamMath.pinchFor(5, 1f, 1.5f) > 1f)    // cockpit: the same, it rides on the car
        assertTrue(CamMath.rigid(5) && CamMath.rigid(1) && CamMath.rigid(2) && !CamMath.rigid(4))
        assertEquals(CamMath.DIST_MAX, CamMath.pinchFor(0, 2.4f, 0.1f), 0f)
        assertEquals(1f, CamMath.pinchFor(0, 1f, Float.NaN), 0f); assertEquals(1f, CamMath.pinchFor(3, 1f, -2f), 0f)
    }
    @Test fun bothFingersUpRaiseTheCameraAndTheGazeTiltsInTheRigidViews() {
        assertTrue(CamMath.height(0, 0f, -100f, 2.6f) > 1f)     // up on the screen = negative dy = higher
        assertTrue(CamMath.height(0, 0f, 100f, 2.6f) < -1f)
        assertTrue(CamMath.height(1, 0f, -100f, 2.6f) in 0.1f..1f)
        assertEquals(6f, CamMath.height(0, 5.9f, -10000f, 2.6f), 0f)
        assertEquals(2f, CamMath.height(3, 2f, Float.NaN, 2.6f), 0f)
        assertEquals(CamMath.height(0, 0f, -100f, 2.6f), CamMath.height(0, 0f, -100f, 2.6f), 0f)
        assertTrue("the same swipe moves a denser screen's camera by the same amount", Math.abs(CamMath.height(0, 0f, -100f, 2.6f) - CamMath.height(0, 0f, -200f, 5.2f)) < 1e-4f)
    }
    @Test fun theFingerDragCanBeInverted() {
        assertEquals(Pair(1f, 1f), CamMath.dragSigns(0)); assertEquals(Pair(-1f, 1f), CamMath.dragSigns(1))
        assertEquals(Pair(1f, -1f), CamMath.dragSigns(2)); assertEquals(Pair(-1f, -1f), CamMath.dragSigns(3))
        assertEquals("Both", CamMath.invertName(3, true)); assertEquals("Нет", CamMath.invertName(-5, false)); assertEquals("Обе", CamMath.invertName(99, false))
    }
}
