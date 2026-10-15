package org.firecracking.firelabel.core;

/** Tells whether the straight line from the camera to a world-space point is blocked. */
public interface Occluder {
    boolean blocked(double[] cameraPos, double[] target);
}
