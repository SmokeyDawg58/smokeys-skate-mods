//! The WorldPainter key changes the challenge system answers. `LayerMan::Update` (`0x82ceb108`)
//! looks every event layer up at the local actor's x/z each simulation update and, when a layer's
//! key differs from `mKeyCache`'s, stores it and sends `cMsgWorldPainter_KeyChange` (layer, key),
//! flushed at once: the film manager answers the security guard areas, the service manager the
//! services.

use super::system::{ChallengeSystem, Event, Field, Message, World};

/// `WorldPainter::eLayerID` of the layers the challenge system answers.
pub mod layer {
    pub const SECURITY_GUARD_AREAS: usize = 6;
    pub const SERVICES: usize = 13;
}

/// `LayerMan::mKeyCache`'s key of each layer the challenge system answers, and the film manager's
/// `IsInNoSkateZone`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldPainterKeys {
    security_guard_areas: u64,
    services: u64,
    /// `cFilmManager::IsInNoSkateZone` (cleared by its constructor `0x82da9c48`).
    pub in_no_skate_zone: bool,
}

/// `cFilmManager::WorldPainterKeyChangeMessageHandler` (`0x82da7838`): entering a security guard
/// area (a nonzero key) sends `cMsgSecurityEnterNoSkate`, leaving one `cMsgSecurityExitNoSkate`,
/// both for player 0, and keeps whether the skater is in one.
fn film_manager_key_change(world: &mut World, key: u64) {
    let (name, inside) = if key != 0 { ("cMsgSecurityEnterNoSkate", true) } else { ("cMsgSecurityExitNoSkate", false) };
    world.send(Event::Challenge(Message::new(name, vec![("PlayerID", Field::Number(0.0))])));
    world.world_painter.in_no_skate_zone = inside;
}

/// `cServiceManager::WorldPainterKeyChangeMessageHandler` (`0x8289d390`): a service area's key
/// (its service challenge's key) sends `cMsgWorldPainterServiceNotification`, leaving one
/// `cMsgWorldPainterLeftServiceNotification`.
fn service_manager_key_change(world: &mut World, key: u64) {
    let message = if key != 0 {
        Message::new("cMsgWorldPainterServiceNotification", vec![("ServiceKey", Field::Key(key))])
    } else {
        Message::new("cMsgWorldPainterLeftServiceNotification", Vec::new())
    };
    world.send(Event::Challenge(message));
}

/// ServiceManager.OnTimerExpired8289d198 repeats both lookups after a main
/// challenge. LayerMan has sampled these keys at the current local position.
pub(crate) fn reemit_service_notifications(world: &mut World) {
    film_manager_key_change(world, world.world_painter.security_guard_areas);
    service_manager_key_change(world, world.world_painter.services);
}

impl ChallengeSystem {
    /// `LayerMan::Update` for the security guard areas' and the services' keys at the local
    /// actor: each changed key is cached and its handler runs.
    pub fn world_painter_update(&mut self, security_guard_areas: u64, services: u64) {
        let world = &mut self.world;
        if world.world_painter.security_guard_areas != security_guard_areas {
            world.world_painter.security_guard_areas = security_guard_areas;
            film_manager_key_change(world, security_guard_areas);
        }
        if world.world_painter.services != services {
            world.world_painter.services = services;
            service_manager_key_change(world, services);
        }
    }
}
