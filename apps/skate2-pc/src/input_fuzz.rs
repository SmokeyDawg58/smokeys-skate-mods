//! Seeded random pad input for stress runs (`SKATE2_DIAGNOSTIC_FUZZ=<seed>`).
//! Each control re-rolls on its own random interval; Start, Back and the
//! D-pad are never pressed, so the session stays in play.
//! `SKATE2_DIAGNOSTIC_FUZZ_FROM=<tick>` holds the pad neutral until that
//! completed tick, so a scripted run's menus and joins go undisturbed.
use skate2_core::XboxPadSample;

struct Hold<T> {
    value: T,
    until: u64,
}

pub(crate) struct InputFuzz {
    state: u64,
    left: Hold<[i16; 2]>,
    right: Hold<[i16; 2]>,
    triggers: Hold<[u8; 2]>,
    buttons: Hold<u16>,
    from: u64,
}

impl InputFuzz {
    pub(crate) fn from_env() -> Option<Self> {
        let seed = std::env::var("SKATE2_DIAGNOSTIC_FUZZ")
            .ok()?
            .parse::<u64>()
            .ok()?;
        eprintln!("diagnostic input fuzz seed={seed}");
        Some(Self {
            state: seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            left: Hold {
                value: [0; 2],
                until: 0,
            },
            right: Hold {
                value: [0; 2],
                until: 0,
            },
            triggers: Hold {
                value: [0; 2],
                until: 0,
            },
            buttons: Hold { value: 0, until: 0 },
            from: std::env::var("SKATE2_DIAGNOSTIC_FUZZ_FROM").ok().and_then(|tick| tick.trim().parse().ok()).unwrap_or(0),
        })
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn stick(&mut self, neutral: u64, compass: u64) -> [i16; 2] {
        let roll = self.below(100);
        if roll < neutral {
            [0, 0]
        } else if roll < neutral + compass {
            let angle = self.below(8) as f32 * std::f32::consts::FRAC_PI_4;
            [
                (angle.cos() * 32767.0) as i16,
                (angle.sin() * 32767.0) as i16,
            ]
        } else {
            [self.next() as i16, self.next() as i16]
        }
    }

    pub(crate) fn sample(&mut self, tick: u64) -> XboxPadSample {
        if tick < self.from {
            return XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
        }
        if tick >= self.left.until {
            self.left.value = self.stick(30, 40);
            self.left.until = tick + 8 + self.below(82);
        }
        if tick >= self.right.until {
            self.right.value = self.stick(35, 45);
            self.right.until = tick + 1 + self.below(13);
        }
        if tick >= self.triggers.until {
            self.triggers.value = match self.below(100) {
                0..60 => [0, 0],
                60..75 => [u8::MAX, 0],
                75..90 => [0, u8::MAX],
                _ => [u8::MAX, u8::MAX],
            };
            self.triggers.until = tick + 10 + self.below(70);
        }
        if tick >= self.buttons.until {
            let mut buttons = 0;
            if self.below(100) >= 50 {
                for (mask, percent) in [
                    (0x1000, 45),
                    (0x4000, 30),
                    (0x2000, 20),
                    (0x0100, 10),
                    (0x0200, 10),
                    (0x8000, 3),
                ] {
                    if self.below(100) < percent {
                        buttons |= mask;
                    }
                }
            }
            self.buttons.value = buttons;
            self.buttons.until = tick + 2 + self.below(38);
        }
        XboxPadSample {
            buttons: self.buttons.value,
            left_trigger: self.triggers.value[0],
            right_trigger: self.triggers.value[1],
            left_x: self.left.value[0],
            left_y: self.left.value[1],
            right_x: self.right.value[0],
            right_y: self.right.value[1],
        }
    }
}
