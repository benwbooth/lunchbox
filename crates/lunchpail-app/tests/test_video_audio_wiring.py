"""Guard the shared preference wiring in the full application QML."""
from pathlib import Path
import re
import unittest


class VideoAudioWiring(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.qml = (Path(__file__).resolve().parents[1] / "qml" / "Main.qml").read_text()

    def test_every_speaker_control_changes_the_same_preference(self):
        self.assertNotIn("hoverPreviewAudioMuted", self.qml)
        self.assertIn("muted: root.videoAudioMuted", self.qml)
        self.assertIn("unmuted: !root.videoAudioMuted", self.qml)
        self.assertEqual(
            re.findall(r"root\.videoAudioMuted\s*=\s*([^\n]+)", self.qml),
            ["!root.videoAudioMuted"] * 3,
        )
        self.assertNotRegex(self.qml, r"gameVideoAudio\.muted\s*=")

    def test_unmute_rebuild_preserves_details_pause_state(self):
        self.assertIn("gameVideoPlayer.enableAudio()", self.qml)
        self.assertIn("unmuteResumePlaying = playbackState === MediaPlayer.PlayingState", self.qml)
        self.assertRegex(self.qml, r"if \(unmuteResumePlaying\) play\(\)\s+else pause\(\)")


if __name__ == "__main__":
    unittest.main()
