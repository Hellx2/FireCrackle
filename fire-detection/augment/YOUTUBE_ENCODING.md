# What YouTube does to an upload (research notes, 2026-10-15)

Goal: make our compression augmentation resemble what really reaches viewers and crackers, so a detector trained
or benchmarked on it holds up on real YouTube footage. **YouTube does not publish what it serves.** What follows is
what could be established from public sources, kept apart from what is only an assumption.

## Established (with sources)

- **Upload recommendations** (not what is delivered): MP4, H.264 High profile, progressive, 2 consecutive B-frames,
  closed GOP, GOP = half the frame rate, CABAC, 4:2:0, variable bitrate, upload at the recorded frame rate. SDR
  bitrates: 1080p 8 Mbps (24-30 fps) / 12 Mbps (48-60 fps), 720p 5 / 7.5, 2160p 35-45 / 53-68.
  Source: [YouTube recommended upload encoding settings](https://support.google.com/youtube/answer/1722171?hl=en).
- **Everything is re-encoded.** Videos are transcoded to a ladder of resolutions and codecs regardless of what was
  uploaded; the viewer gets one of those renditions.
- **Codecs served**: H.264 (the ladder stops at 1080p), VP9, and AV1 (full ladders incl. 1440p/2160p on popular or
  high-resolution videos). Which codec a video gets has depended on view count and has shifted over time; a 2022
  re-check found H.264 and VP9 for every video tested, AV1 pushed to ever lower view counts. Sources:
  [Which Codecs Does YouTube Use?](https://streaminglearningcenter.com/codecs/which-codecs-does-youtube-use.html),
  [Which Codec Does YouTube Use, Part Deux](https://streaminglearningcenter.com/?p=16297),
  [How YouTube encodes videos](https://www.streamingmedia.com/Articles/Columns/The-Producers-View/How-YouTube-Encodes-Videos-150212.aspx).
- **Relative efficiency at equal quality**: VP9 about 15% less bandwidth than H.264, AV1 about 45% less (one author's
  tests); at 1080p AV1 47% smaller than H.264 and 21% smaller than VP9 (YouTube-served files, one author).
- **Measured delivered bitrates (single videos, so only anchor points)**: a 1080p video listed 2646 kbps (VP9) and
  5055 kbps (H.264); a 1080p60 case from a 4K upload had VP9 6427 kbps vs AVC 5899 kbps. YouTube uses per-title
  encoding, so bitrate depends heavily on the content (fast, noisy game footage costs more bits). Source:
  [VideoHelp thread](https://forum.videohelp.com/showthread.php?p=2608917) and the Streaming Media articles above.
  "Premium 1080p" is a separate, roughly 13 Mbps tier.
- **Live** ingest is different (CBR, 2 s keyframes) and is not what a normal upload goes through.

## Not established (do not trust numbers for these)

- The delivered bitrate per rung for each codec, in general (only the anchor points above).
- Keyframe interval, B-frame count, profile/level, rate-control mode of the *delivered* streams.
- Which codec a given video will be served at a given moment.

## Measured: does the sync strip survive these chains? (2026-10-15)

`python robustness.py --frames 90 --seeds 2` on a synthetic, noisy 1080p30 stamped video, 2 sampled chains per
(codec, rung), real x264 / libvpx-vp9 / SVT-AV1 encodes with the ASSUMED ranges. Full data:
`results/robustness-2026-10-15.json`. Share of frames that decode to the correct tick and frame id:

| rung | H.264 | VP9 | AV1 |
|---|---|---|---|
| 1080p | 100% | 100% | 100% |
| 720p | 100% | 100% | 98.3% |
| 480p | 99.4% | 100% | 98.9% |
| 360p | 100% | 100% | 88.9% |
| 240p, 144p | not decodable: video narrower than 480 px (by design) | | |

**No frame ever decoded to a wrong id**: every loss is an "undecodable" frame (the checksum and the ambiguity
rule reject damaged strips instead of guessing). The weakest case is AV1 at the very low end of its assumed bitrate
range (286 kbps at 360p). Caveats: the source is synthetic (heavy grain, harder than most game footage, but not
real footage), only 2 chains per cell, and the bitrates are the assumed ranges, so this shows the margin, not the
exact YouTube behaviour. Rerun after calibrating.

## What we do about it

1. **Wide, anchored ranges.** `profiles.py` samples codec, rung and bitrate from ranges that contain the anchor
   points above and extend well past them. Wide on purpose: it is better to over-cover than to claim precision
   we do not have. Every range is marked ASSUMPTION in the code and in each manifest.
2. **Two stages**, like the real path: recorder (OBS-like x264) -> YouTube-like re-encode (+ rescale to a ladder
   rung, never upscaling) -> decode.
3. **Calibrate against the real thing.** The accurate method is to measure what YouTube returns for OUR footage:
   upload one test recording as *unlisted*, download the processed streams yourself (`yt-dlp -F URL`, then pick the
   formats), and run `python augment/calibrate.py FILE...`. It prints codec, profile, level, bitrate, keyframe
   interval, B-frame ratio, pixel format and range, and the nearest ladder rung. Put the measured ranges into
   `augment/profiles_calibrated.json`; they then override the assumptions. That upload is outward-facing, so it is
   left to you; nothing here uploads or downloads YouTube content.
4. **Know the limit of the strip.** The sync strip needs 480 px of video width, so YouTube's 360p (640 wide) rung
   still decodes but 240p (426 wide) and below do not. Crackers use high rungs, so this is acceptable, and the
   decoder reports such frames as undecodable instead of guessing.
