package org.firecracking.firelabel.core;

/** Which texels of the currently displayed sprite frame are visible (not fully transparent). */
public interface TexelMask {
    boolean opaque(int tx, int ty);
}
