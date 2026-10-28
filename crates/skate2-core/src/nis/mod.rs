//! In-game cutscenes (NIS): `Sk8::NISManager` on the simulation side and the
//! `Sk8::Scene` scene manager, player and ICE camera on the presentation side.
//! See `docs/career/HANDOFF.md`.

pub mod audio;
pub mod back_end;
pub mod camera;
pub mod characters;
pub mod directory;
pub mod fe_overlays;
pub mod manager;
pub mod subtitles;
pub mod transition;
