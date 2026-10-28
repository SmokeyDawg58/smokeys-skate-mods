//! The full-map viewport used by the FELocation browser. Interactive pan/zoom
//! has its own native state machine and is not approximated here.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewport {
    pub x_min: i32,
    pub x_max: i32,
    pub y_min: i32,
    pub y_max: i32,
    pub texture_width: i32,
    pub texture_height: i32,
}

impl MapViewport {
    /// ResetMap82617b88 with kMapDefaultZoom's source default1.0 (82f587e8).
    pub const FULL_MAP_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    /// SetMapInfo82606f08: aspect multipliers are distinct from world extents.
    #[must_use]
    pub fn aspect(self) -> [f32; 2] {
        let width = self.texture_width as f32;
        let height = self.texture_height as f32;
        let sx = width * f32::from_bits(0x3b09_f874);
        let sy = height * f32::from_bits(0x3b09_f874);
        if sx == sy {
            [1.0, 1.0]
        } else if sx < sy {
            [width / height, 1.0]
        } else {
            [1.0, height / width]
        }
    }

    /// GetMapCenter82617648 maps STANDARD_UV's midpoint using original
    /// integer world bounds. The world pair is X/Z, with Z increasing down
    /// the authored map image; no host-axis sign substitution occurs.
    #[must_use]
    pub fn world_center(self) -> [f32; 2] {
        [
            ((self.x_max - self.x_min) as f32).mul_add(0.5, self.x_min as f32),
            ((self.y_max - self.y_min) as f32).mul_add(0.5, self.y_min as f32),
        ]
    }

    /// GetScreenCoordFromWorldCoord826178c0 returns center-relative map
    /// texture pixels. Scaling defaults originate in82f588a8/82f58908;
    /// callers supply the current native cvar values when exposing overrides.
    #[must_use]
    pub fn screen_offset(self, world: [f32; 2], icon_scale: [f32; 2]) -> [f32; 2] {
        let center = self.world_center();
        let top_left = [self.x_min as f32, self.y_min as f32];
        let x_factor = (self.texture_width as f32 * 0.5) / (center[0] - top_left[0]);
        let y_factor = (self.texture_height as f32 * 0.5) / (center[1] - top_left[1]);
        [
            (icon_scale[0] * (world[0] - center[0])) * x_factor,
            (icon_scale[1] * (world[1] - center[1])) * y_factor,
        ]
    }

    pub const DEFAULT_ICON_SCALE: [f32; 2] = [0.99, 0.95];
}
