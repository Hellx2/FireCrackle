package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class Crc8Test {
    @Test
    void standardCheckValue() {
        assertEquals(0xF4, Crc8.crc8("123456789".getBytes(StandardCharsets.US_ASCII)));
    }

    @Test
    void emptyInputIsZero() {
        assertEquals(0, Crc8.crc8(new byte[0]));
    }

    @Test
    void resultIsAlwaysAnUnsignedByte() {
        for (int i = 0; i < 256; i++) {
            int c = Crc8.crc8(new byte[] {(byte) i, (byte) (i * 7), (byte) 0xFF});
            assertEquals(c & 0xFF, c);
        }
    }

    @Test
    void highBytesAreNotSignExtended() {
        // 0xFF as a negative Java byte must behave like 255
        assertEquals(Crc8.crc8(new byte[] {(byte) 0x80}), Crc8.crc8(new byte[] {-128}));
        assertEquals(0xF3, Crc8.crc8(new byte[] {(byte) 0xFF}));
    }
}
