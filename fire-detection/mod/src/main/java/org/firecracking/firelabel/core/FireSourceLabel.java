package org.firecracking.firelabel.core;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * One visible fire source in one frame: a fire block, a burning entity, or the first-person on-fire overlay.
 * It has ONE box (the union of its parts) and the texture variants it shows as {@link FirePart}s.
 */
public final class FireSourceLabel {
    public enum Source {
        BLOCK,
        ENTITY,
        OVERLAY
    }

    public final Source source;
    /** Block position (BLOCK only). */
    public final int x;
    public final int y;
    public final int z;
    /** Entity id and type (ENTITY only). */
    public final int entityId;
    public final String entityType;
    /** Camera distance in blocks (0 for the overlay). */
    public final double dist;

    private final BBox box;
    private final double vis;
    private final double veil;
    private final List<FirePart> parts;
    private final List<FireQuadLabel> quads;

    private FireSourceLabel(
            Source source, int x, int y, int z, int entityId, String entityType, double dist, List<FirePart> parts,
            List<FireQuadLabel> quads) {
        if (parts == null || parts.isEmpty()) {
            throw new IllegalArgumentException("a fire source needs at least one part");
        }
        if (quads == null) {
            throw new IllegalArgumentException("quads must not be null (use an empty list)");
        }
        for (FireQuadLabel q : quads) {
            if (q == null) {
                throw new IllegalArgumentException("a quad entry is null");
            }
        }
        if (Double.isNaN(dist) || Double.isInfinite(dist) || dist < 0.0) {
            throw new IllegalArgumentException("dist must be finite and >= 0: " + dist);
        }
        List<FirePart> copy = new ArrayList<FirePart>(parts);
        BBox union = new BBox();
        long visible = 0;
        long opaque = 0;
        double veilSum = 0.0;
        for (int i = 0; i < copy.size(); i++) {
            FirePart p = copy.get(i);
            for (int j = 0; j < i; j++) {
                if (copy.get(j).sheet == p.sheet && copy.get(j).mirrored == p.mirrored) {
                    throw new IllegalArgumentException("two parts with the same sheet and mirror");
                }
            }
            union.add(p.box());
            visible += p.visibleTexels;
            opaque += p.opaqueTexels;
            veilSum += p.veil * p.visibleTexels;
        }
        this.source = source;
        this.x = x;
        this.y = y;
        this.z = z;
        this.entityId = entityId;
        this.entityType = entityType;
        this.dist = dist;
        this.box = union;
        this.vis = (double) visible / opaque;
        this.veil = veilSum / visible; // visible texel weighted; at least one part with >= 1 visible texel exists
        this.parts = Collections.unmodifiableList(copy);
        this.quads = Collections.unmodifiableList(new ArrayList<FireQuadLabel>(quads));
    }

    public static FireSourceLabel block(int x, int y, int z, double dist, List<FirePart> parts) {
        return block(x, y, z, dist, parts, Collections.<FireQuadLabel>emptyList());
    }

    public static FireSourceLabel block(int x, int y, int z, double dist, List<FirePart> parts, List<FireQuadLabel> quads) {
        return new FireSourceLabel(Source.BLOCK, x, y, z, 0, null, dist, parts, quads);
    }

    public static FireSourceLabel entity(int entityId, String entityType, double dist, List<FirePart> parts) {
        return entity(entityId, entityType, dist, parts, Collections.<FireQuadLabel>emptyList());
    }

    public static FireSourceLabel entity(
            int entityId, String entityType, double dist, List<FirePart> parts, List<FireQuadLabel> quads) {
        if (entityType == null) {
            throw new IllegalArgumentException("entity type is required");
        }
        return new FireSourceLabel(Source.ENTITY, 0, 0, 0, entityId, entityType, dist, parts, quads);
    }

    public static FireSourceLabel overlay(List<FirePart> parts) {
        return overlay(parts, Collections.<FireQuadLabel>emptyList());
    }

    public static FireSourceLabel overlay(List<FirePart> parts, List<FireQuadLabel> quads) {
        return new FireSourceLabel(Source.OVERLAY, 0, 0, 0, 0, null, 0.0, parts, quads);
    }

    /** The one box of this source (union of its parts). A copy: callers cannot change the label. */
    public BBox box() {
        return box.copy();
    }

    /** Visible opaque texels / opaque texels over all parts, 0..1. */
    public double vis() {
        return vis;
    }

    /** Mean GUI alpha over all visible texels of this source (visible-texel weighted over its parts), 0..1. */
    public double veil() {
        return veil;
    }

    /** The quads drawn for this source in draw order (later ones over earlier ones); empty if none were recorded. */
    public List<FireQuadLabel> quads() {
        return quads;
    }

    public List<FirePart> parts() {
        return parts;
    }
}
