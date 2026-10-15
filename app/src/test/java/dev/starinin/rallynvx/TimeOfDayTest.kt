package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class TimeOfDayTest {
    @Test fun `the modes cycle and come back to Auto`() {
        var m = 0
        val seen = (1..TimeOfDay.COUNT).map { m = TimeOfDay.next(m); m }
        assertEquals(listOf(1, 2, 3, 0), seen)
        assertEquals(0, TimeOfDay.next(-1 + 1 + 3))
    }
    @Test fun `Auto is negative and the others are phases of the day`() {
        assertTrue(TimeOfDay.phase(0) < 0f)
        for (m in 1 until TimeOfDay.COUNT) assertTrue(TimeOfDay.phase(m) in 0f..1f)
        assertEquals(0.5f, TimeOfDay.phase(1), 0f)   // Day is noon
        assertEquals(0f, TimeOfDay.phase(3), 0f)     // Night is midnight
    }
    @Test fun `names exist in both languages`() {
        for (m in 0 until TimeOfDay.COUNT) { assertTrue(TimeOfDay.name(m, true).isNotBlank()); assertTrue(TimeOfDay.name(m, false).isNotBlank()) }
    }
}
