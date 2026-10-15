package org.firecracking.firelabel.core;

/** Render-frame id source. Wraps to 0 after {@link StripPattern#MAX_ID}. */
public final class FrameCounter {
    private int next;

    public FrameCounter() {
        this(0);
    }

    public FrameCounter(int start) {
        if (start < 0 || start > StripPattern.MAX_ID) {
            throw new IllegalArgumentException("start out of range: " + start);
        }
        this.next = start;
    }

    public int next() {
        int id = next;
        next = id == StripPattern.MAX_ID ? 0 : id + 1;
        return id;
    }
}
