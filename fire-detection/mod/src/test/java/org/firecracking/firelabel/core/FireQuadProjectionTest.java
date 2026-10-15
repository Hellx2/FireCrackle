package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

class FireQuadProjectionTest {
    private static final double[][] UV = {{0,0},{1,0},{1,1},{0,1}};
    private static QuadVisibility.Result visible() {
        QuadVisibility.Result r = new QuadVisibility.Result();
        r.opaqueTexels=100; r.visibleTexels=40; r.veilSum=10;
        return r;
    }
    private static Projector projector() {
        return new Projector(ProjectorTest.perspective(90,1,0.1,100),100,100);
    }
    @Test void recordsFullPlaneInFramebufferPixelsIncludingOutsideTheScreen() {
        // Camera-relative quad spans x=-3..1 at z=-2: screen x=-25..75.
        Quad q=new Quad(new double[][]{{7,1,-2},{11,1,-2},{11,-1,-2},{7,-1,-2}},UV);
        FireQuadLabel label=FireQuadProjection.capture(1,12,q,new double[]{10,0,0},projector(),visible());
        assertArrayEquals(new double[]{-25,25},label.corner(0),1e-8);
        assertArrayEquals(new double[]{75,75},label.corner(2),1e-8);
        assertArrayEquals(UV[3],label.uv(3));
        assertEquals(1,label.sheet); assertEquals(12,label.row);
        assertEquals(0.4,label.vis(),1e-9); assertEquals(0.25,label.veil,1e-9);
    }
    @Test void behindCameraCornerMakesGeometryNullWithoutInventingScreenCoordinates() {
        Quad q=new Quad(new double[][]{{-1,1,1},{1,1,-2},{1,-1,-2},{-1,-1,-2}},UV);
        FireQuadLabel label=FireQuadProjection.capture(0,3,q,new double[]{0,0,0},projector(),visible());
        assertFalse(label.hasCorners()); assertArrayEquals(UV[0],label.uv(0));
    }
    @Test void invisibleQuadIsOmitted() {
        Quad q=new Quad(new double[][]{{-1,1,-2},{1,1,-2},{1,-1,-2},{-1,-1,-2}},UV);
        assertNull(FireQuadProjection.capture(0,3,q,new double[]{0,0,0},projector(),new QuadVisibility.Result()));
    }
    @Test void projectedQuadReachesTheSerializedLog() {
        Quad q=new Quad(new double[][]{{-1,1,-2},{1,1,-2},{1,-1,-2},{-1,-1,-2}},UV);
        FireQuadLabel label=FireQuadProjection.capture(0,3,q,new double[]{0,0,0},projector(),visible());
        BBox box=new BBox(); box.add(25,25); box.add(75,75);
        FirePart part=new FirePart(0,label.mirrored,3,box,40,100);
        FireSourceLabel source=FireSourceLabel.block(0,0,0,2,Arrays.asList(part),Arrays.asList(label));
        String json=LabelLog.format(new FrameRecord(1,2,3,19,100,100,Arrays.asList(source)));
        assertTrue(json.contains("\"quads\":[{"));
        assertTrue(json.contains("\"c\":[25.0,25.0,75.0,25.0,75.0,75.0,25.0,75.0]"));
    }
}
