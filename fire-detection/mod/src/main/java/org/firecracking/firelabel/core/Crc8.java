package org.firecracking.firelabel.core;

/** CRC-8, poly 0x07, init 0, no reflection, no xorout. Must match core/src/sync.rs. */
public final class Crc8 {
    private Crc8() {}

    public static int crc8(byte[] data) {
        int crc = 0;
        for (byte b : data) {
            crc ^= b & 0xFF;
            for (int i = 0; i < 8; i++) {
                crc = (crc & 0x80) != 0 ? ((crc << 1) ^ 0x07) & 0xFF : (crc << 1) & 0xFF;
            }
        }
        return crc;
    }
}
