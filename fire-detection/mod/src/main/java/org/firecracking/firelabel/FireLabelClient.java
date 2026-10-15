package org.firecracking.firelabel;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public class FireLabelClient implements ClientModInitializer {
    private static final Logger LOGGER = LogManager.getLogger("firelabel");

    @Override
    public void onInitializeClient() {
        ClientTickEvents.END_CLIENT_TICK.register(client -> FireLabel.onClientTick());
        ClientLifecycleEvents.CLIENT_STOPPING.register(client -> FireLabel.onClientStopping());
        if (Boolean.getBoolean("firelabel.debug")) {
            LOGGER.warn("DEBUG OVERLAY ON: coloured boxes are drawn into the frame. Do NOT record footage in this mode "
                    + "(start with -PfirelabelDebug=false).");
        } else {
            LOGGER.info("Recording-safe mode: only the sync strip is drawn into the frame.");
        }
    }
}
