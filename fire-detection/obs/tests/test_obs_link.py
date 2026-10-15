from types import SimpleNamespace

import pytest

from obs_link import ObsLink, ObsLinkError


class FakeClient:
    """Mimics the parts of obsws_python.ReqClient we use."""

    def __init__(self, recording=False, output_path="C:/rec/2026-10-15.mkv"):
        self.recording = recording
        self.output_path = output_path
        self.calls = []
        self.fail_with = None

    def _maybe_fail(self):
        if self.fail_with:
            raise self.fail_with

    def get_record_status(self):
        self.calls.append("status")
        self._maybe_fail()
        return SimpleNamespace(output_active=self.recording)

    def start_record(self):
        self.calls.append("start")
        self._maybe_fail()
        self.recording = True

    def stop_record(self):
        self.calls.append("stop")
        self._maybe_fail()
        self.recording = False
        return SimpleNamespace(output_path=self.output_path)


# ---------- basics ----------

def test_is_recording_reflects_obs_state():
    assert ObsLink(FakeClient(recording=False)).is_recording() is False
    assert ObsLink(FakeClient(recording=True)).is_recording() is True


def test_start_then_stop_returns_output_path():
    client = FakeClient()
    link = ObsLink(client)
    link.start_recording()
    assert link.is_recording() is True
    assert link.stop_recording() == "C:/rec/2026-10-15.mkv"
    assert link.is_recording() is False
    assert client.calls.count("start") == 1
    assert client.calls.count("stop") == 1


def test_connect_passes_settings_to_the_client_factory():
    seen = {}

    def factory(**kwargs):
        seen.update(kwargs)
        return FakeClient()

    link = ObsLink.connect(host="10.0.0.5", port=4466, password="pw", client_factory=factory)
    assert isinstance(link, ObsLink)
    assert seen["host"] == "10.0.0.5"
    assert seen["port"] == 4466
    assert seen["password"] == "pw"


# ---------- failures ----------

def test_connect_when_obs_is_down_gives_a_clear_error():
    def factory(**kwargs):
        raise ConnectionRefusedError("refused")

    with pytest.raises(ObsLinkError, match="OBS"):
        ObsLink.connect(client_factory=factory)


def test_start_while_already_recording_is_an_error_and_does_not_call_start():
    client = FakeClient(recording=True)
    with pytest.raises(ObsLinkError, match="already"):
        ObsLink(client).start_recording()
    assert "start" not in client.calls


def test_stop_while_idle_is_an_error_and_does_not_call_stop():
    client = FakeClient(recording=False)
    with pytest.raises(ObsLinkError, match="not recording"):
        ObsLink(client).stop_recording()
    assert "stop" not in client.calls


@pytest.mark.parametrize("path", [None, ""])
def test_stop_without_an_output_path_is_an_error(path):
    client = FakeClient(recording=True, output_path=path)
    with pytest.raises(ObsLinkError, match="output path"):
        ObsLink(client).stop_recording()


def test_client_exceptions_are_wrapped():
    client = FakeClient()
    client.fail_with = OSError("socket closed")
    link = ObsLink(client)
    with pytest.raises(ObsLinkError, match="socket closed"):
        link.is_recording()
    with pytest.raises(ObsLinkError):
        link.start_recording()
