package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class LoadMathTest {
    @Test fun unpacksStageAndPercent() {
        val p = (3 shl 8) or 45
        assertEquals(45, LoadMath.percent(p)); assertEquals(3, LoadMath.stage(p)); assertFalse(LoadMath.done(p))
        assertTrue(LoadMath.done((6 shl 8) or 100))
    }
    @Test fun labelsInBothLanguages() {
        assertEquals("Compiling shaders", LoadMath.label(3 shl 8, true))
        assertEquals("Компиляция шейдеров", LoadMath.label(3 shl 8, false))
        assertEquals("Loading", LoadMath.label(0, true))
    }
    @Test fun aDeadGameIsReported() { assertTrue(LoadMath.failed(99 shl 8)); assertFalse(LoadMath.failed(6 shl 8)); assertTrue(LoadMath.label(99 shl 8, true).startsWith("Startup failed")) }
    @Test fun easeNeverGoesBackOrOvershoots() {
        assertEquals(0.5f, LoadMath.ease(0.5f, 0.3f, 0.1f), 0f)
        val e = LoadMath.ease(0.2f, 0.8f, 0.05f)
        assertTrue(e > 0.2f && e <= 0.8f)
        assertEquals(0.8f, LoadMath.ease(0.2f, 0.8f, 100f), 1e-4f)
    }
}
