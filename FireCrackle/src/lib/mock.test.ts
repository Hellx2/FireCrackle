import { expect, it } from "vitest";
import { createMockBackend } from "./mock";

it("manual forward accepts departing guesses once; playback does not self-label or recalibrate", async () => {
  const b=createMockBackend();
  await b.act({type:"clickCell",col:1,row:5});
  const action=JSON.parse('{"type":"manualStep","frames":3}');
  let v=await b.act(action);
  expect(v.current).toBe(4);expect(v.labelled_count).toBe(2);expect(v.suggestion?.from_frame).toBe(0);
  v=await b.act({type:"step",frames:1});expect(v.labelled_count).toBe(2);
  v=await b.act({type:"undo"});expect(v.current).toBe(1);expect(v.label).toBe("unset");
});
