"""Start/stop OBS recording over obs-websocket v5 and report where the file went."""

import obsws_python


class ObsLinkError(Exception):
    pass


class ObsLink:
    def __init__(self, client):
        self._client = client

    @classmethod
    def connect(cls, host="localhost", port=4455, password="", client_factory=obsws_python.ReqClient):
        try:
            client = client_factory(host=host, port=port, password=password)
        except Exception as e:
            raise ObsLinkError(
                f"Could not connect to OBS at {host}:{port} "
                f"(is OBS running with the WebSocket server enabled?): {e}"
            ) from e
        return cls(client)

    def _call(self, name):
        try:
            return getattr(self._client, name)()
        except Exception as e:
            raise ObsLinkError(f"OBS request {name} failed: {e}") from e

    def is_recording(self):
        return bool(self._call("get_record_status").output_active)

    def start_recording(self):
        if self.is_recording():
            raise ObsLinkError("OBS is already recording")
        self._call("start_record")

    def stop_recording(self):
        if not self.is_recording():
            raise ObsLinkError("OBS is not recording")
        path = self._call("stop_record").output_path
        if not path:
            raise ObsLinkError("OBS stopped recording but returned no output path")
        return path
