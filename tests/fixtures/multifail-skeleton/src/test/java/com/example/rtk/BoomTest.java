package com.example.ptk;

import org.junit.jupiter.api.Test;

class BoomTest {
    @Test
    void boom() {
        throw new IllegalStateException("boom: errors-only class for fixture capture");
    }
}
