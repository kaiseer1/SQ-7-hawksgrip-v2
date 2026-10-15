import pytest

from hawksgrip import Session
from hawksgrip.bridge import LockstepDriver


class FakeSession:
    tick_hz = 120

    def __init__(self):
        self.tick = 0

    def step(self, n: int) -> int:
        self.tick += n
        return self.tick


def test_one_hour_at_60fps_is_exact():
    fs = FakeSession()
    d = LockstepDriver(fs)
    frame_ns = 1_000_000_000 // 60  # 16_666_666 ns: not an exact 60th of a second
    frames = 60 * 3600
    for _ in range(frames):
        d.advance(frame_ns)
    expected = frames * frame_ns * 120 // 1_000_000_000
    assert fs.tick == expected
    assert d.ticks_dropped == 0
    assert 2 * frames - 1 <= fs.tick <= 2 * frames


def test_jittery_frames_still_exact():
    fs = FakeSession()
    d = LockstepDriver(fs, max_ticks_per_frame=1000)
    pattern = [16_000_000, 17_500_000, 15_200_000, 33_000_000, 8_300_000]
    total = 0
    for i in range(10_000):
        dt = pattern[i % len(pattern)]
        total += dt
        d.advance(dt)
    assert fs.tick == total * 120 // 1_000_000_000


def test_overrun_drops_backlog_instead_of_spiralling():
    fs = FakeSession()
    d = LockstepDriver(fs)  # default cap = 30 ticks/frame
    n = d.advance(2_000_000_000)  # a 2-second stall
    assert n == 30
    assert d.ticks_dropped == 240 - 30
    assert d.advance(16_666_667) == 2  # next 60 fps frame: back to normal


def test_time_scale_and_alpha():
    fs = FakeSession()
    d = LockstepDriver(fs, time_scale=2.0, max_ticks_per_frame=10_000)
    d.advance(1_000_000_000)
    assert fs.tick == 240
    d2 = LockstepDriver(FakeSession())
    d2.advance(1_000_000_000 // 240)  # half a tick
    assert d2.alpha == pytest.approx(0.5, abs=1e-6)


def test_rejects_negative_time():
    with pytest.raises(ValueError):
        LockstepDriver(FakeSession()).advance(-1)


def test_drives_real_session():
    s = Session()
    d = LockstepDriver(s)
    for _ in range(60):
        d.advance(1_000_000_000 // 60)
    assert s.tick == 119 or s.tick == 120
