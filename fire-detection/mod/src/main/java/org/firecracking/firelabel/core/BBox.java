package org.firecracking.firelabel.core;

/** Axis-aligned box in pixels that grows to cover points. Starts empty. */
public final class BBox {
    private double x0;
    private double y0;
    private double x1;
    private double y1;
    private boolean empty = true;

    public boolean isEmpty() {
        return empty;
    }

    public double x0() {
        return x0;
    }

    public double y0() {
        return y0;
    }

    public double x1() {
        return x1;
    }

    public double y1() {
        return y1;
    }

    public void add(double x, double y) {
        if (Double.isNaN(x) || Double.isNaN(y) || Double.isInfinite(x) || Double.isInfinite(y)) {
            throw new IllegalArgumentException("non-finite point: " + x + ", " + y);
        }
        if (empty) {
            x0 = x1 = x;
            y0 = y1 = y;
            empty = false;
            return;
        }
        x0 = Math.min(x0, x);
        y0 = Math.min(y0, y);
        x1 = Math.max(x1, x);
        y1 = Math.max(y1, y);
    }

    public void add(BBox other) {
        if (other.empty) {
            return;
        }
        add(other.x0, other.y0);
        add(other.x1, other.y1);
    }

    /** Copy limited to the screen {@code [0, w] x [0, h]}; empty if there is no overlap. */
    public BBox clamped(double w, double h) {
        BBox out = new BBox();
        if (empty) {
            return out;
        }
        double nx0 = Math.max(x0, 0);
        double ny0 = Math.max(y0, 0);
        double nx1 = Math.min(x1, w);
        double ny1 = Math.min(y1, h);
        if (nx0 > nx1 || ny0 > ny1) {
            return out;
        }
        out.add(nx0, ny0);
        out.add(nx1, ny1);
        return out;
    }

    public BBox copy() {
        BBox out = new BBox();
        out.add(this);
        return out;
    }
}
