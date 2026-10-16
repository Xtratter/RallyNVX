package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class HandbrakeMathTest {
    @Test fun theLabelShowsWhetherTheHandbrakeIsOnInBothLanguages() {
        assertEquals("Handbrake", HandbrakeMath.label(false, true)); assertEquals("● Handbrake", HandbrakeMath.label(true, true))
        assertEquals("Ручник", HandbrakeMath.label(false, false)); assertEquals("● Ручник", HandbrakeMath.label(true, false))
    }
}
