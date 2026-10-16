# Third-party materials

The root MIT license applies to original FireCracking/FireCrackle code and documentation. It does not transfer ownership of upstream material or user data.

## Minecraft textures and icon artwork

The fire textures in `FireCrackle/src/assets`, `fire-detection/data/assets` and `fire-detection/core/testdata/*.rgba` are derived from Minecraft 1.16.5 textures. The app's current campfire artwork and derived icons in `FireCrackle/src-tauri/icons`, together with `FireCrackle/public/favicon.png`, are Minecraft-derived artwork. These materials are **not covered by this repository's MIT grant**. Minecraft and its assets belong to Mojang/Microsoft; this is an independent project, not an official Minecraft product.

These fixtures are used to match the game's actual animation and to test that matching. Their presence is not a claim of a separate redistribution license. Review the applicable Minecraft terms and permissions before distributing these assets beyond their permitted use. Changes to source visibility are a separate owner decision; this cleanup does not publish the source repository.

## Minecraft code and Yarn

No Minecraft game source, decompiled Java, game JAR or Yarn mappings are vendored as project source. Research notes describe observed behavior in original prose and identify classes/versions so developers can verify it in their own local environment. Fabric/Loom resolves game and mapping dependencies into ignored local caches. Those downloads are not project-authored or MIT-licensed.

The tracked `fire-detection/mod/gradle/wrapper/gradle-wrapper.jar` is the Gradle bootstrapper, not a Minecraft game JAR. Gradle is separately licensed by its upstream project.

## FFmpeg

Release preparation downloads the Gyan Windows essentials package pinned by `FireCrackle/release/runtime-lock.json`. FFmpeg and FFprobe are separate executable programs, not linked into the app. This build is GPLv3; its original LICENSE, README with build/dependency details, and source links are copied into the packaged `tools` directory. Do not remove those notices or represent FFmpeg as MIT-licensed. The runtime is generated locally and is not committed to this repository.

Linux packages use the system FFmpeg/FFprobe installed through the user's distribution; they do not redistribute the Windows runtime. The Debian package declares FFmpeg as a dependency. Distribution-provided libraries and tools retain their own license terms.

## Libraries and build tools

React, Tauri, Rust crates, Python packages, Fabric/Loom and Gradle retain their own licenses. Exact JavaScript/Rust versions are recorded in their lockfiles; Python requirements and Gradle coordinates are recorded with the corresponding tools. Check their upstream notices when redistributing dependencies.

## Recordings and experiment outputs

Recordings, labels and measured benchmark tables may contain personal paths or content owned by their creators. Generated/private datasets are not part of the source distribution. Published research summaries state their inputs and limitations; they do not grant access to private recordings.
