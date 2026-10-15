package org.firecracking.firelabel.core;

/** What the GUI drew over the world, in top-left framebuffer pixels. */
public interface ScreenMask {
    /** GUI alpha at the pixel: 0 = nothing drawn over the world, 1 = opaque, in between = a translucent veil. */
    double alpha(double x, double y);
}
