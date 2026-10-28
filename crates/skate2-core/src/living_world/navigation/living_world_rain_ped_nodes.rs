//! Authored pedestrian sidewalk/crosswalk topology. No inferred connections.
use super::asset::Pin;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SubNode {
    pub(crate) id: u64,
    pub(crate) pin0: u32,
    pub(crate) pin1: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NodeType {
    #[default]
    Uninitialized,
    Crosswalk,
    Road,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PedRainNode {
    pub(crate) kind: NodeType,
    pub(crate) this_crosswalk: SubNode,
    pub(crate) this_road: SubNode,
    pub(crate) next_crosswalk: [SubNode; 2],
    pub(crate) previous_crosswalk: [SubNode; 2],
    pub(crate) incoming_road: [SubNode; 2],
    pub(crate) outgoing_road: [SubNode; 2],
    pub(crate) reversed: bool,
}
impl PedRainNode {
    /// GetStart82e52e68/GetEnd82e530b0. The authored sidewalk is beyond the
    /// second road edge by one metre plus active parking width. Missing streamed
    /// owners return the original (0,1000,0,0) sentinel, not a guessed waypoint.
    pub(crate) fn endpoint(
        &self,
        rain: &mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        end: bool,
    ) -> Result<[f32; 4], super::network::RainNetworkError> {
        use super::network::RainNetworkError;
        let missing = [0., 1000., 0., 0.];
        match self.kind {
            NodeType::Uninitialized => Ok(missing),
            NodeType::Crosswalk => {
                let Some(intersection) = rain.intersection(self.this_crosswalk.id, mesh)? else {
                    return Ok(missing);
                };
                let pin = if end {
                    self.this_crosswalk.pin1
                } else {
                    self.this_crosswalk.pin0
                };
                if pin >= 4 {
                    return Err(RainNetworkError::PinIndex(pin));
                }
                Ok(intersection.record.vector(pin as usize * 16)?)
            }
            NodeType::Road => {
                let Some(road) = rain.roads.get(&self.this_road.id) else {
                    return Ok(missing);
                };
                let segment = if end {
                    road.segments.last()
                } else {
                    road.segments.first()
                }
                .ok_or(RainNetworkError::MissingItem(self.this_road.id))?;
                let offset = if end { 0x50 } else { 0x70 };
                let first = segment.record.vector(offset)?;
                let second = segment.record.vector(offset + 16)?;
                let delta: [f32; 4] = std::array::from_fn(|i| second[i] - first[i]);
                let direction = crate::reckoning_numeric::try_normalize_unsafe(
                    delta,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                )
                .map_err(RainNetworkError::Numeric)?;
                let padding = rain
                    .super_roads
                    .get(&road.record.id(0x98)?)
                    .map_or(0., |road| {
                        1. + if road.data.flags & 1 != 0 {
                            road.data.parking_width
                        } else {
                            0.
                        }
                    });
                Ok(std::array::from_fn(|i| {
                    direction[i].mul_add(padding, first[i] + delta[i])
                }))
            }
        }
    }

    /// IsValid82e52db0. Resolve streamed owners each time; IDs survive unloads.
    pub(crate) fn is_valid(
        &self,
        rain: &mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
    ) -> Result<bool, super::network::RainNetworkError> {
        Ok(match self.kind {
            NodeType::Uninitialized => false,
            NodeType::Road => rain.roads.contains_key(&self.this_road.id),
            NodeType::Crosswalk => rain.intersection(self.this_crosswalk.id, mesh)?.is_some(),
        })
    }

    /// GetNext/PrevRoad[12]82e53320..53620. Native missing links return the
    /// zero-initialized node, never a synthesized geometric connection.
    pub(crate) fn adjacent_road(
        &self,
        rain: &super::RainMan,
        next: bool,
        branch: usize,
    ) -> Result<Self, super::network::RainNetworkError> {
        let links = if next {
            &self.outgoing_road
        } else {
            &self.incoming_road
        };
        let link = links
            .get(branch)
            .ok_or(super::network::RainNetworkError::PinIndex(branch as u32))?;
        Ok(rain
            .roads
            .get(&link.id)
            .map_or(Self::default(), |road| road.ped))
    }

    /// GetNext/PrevCrossWalk[12]82e53720..53ab0.
    pub(crate) fn adjacent_crosswalk(
        &self,
        rain: &mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        next: bool,
        branch: usize,
    ) -> Result<Self, super::network::RainNetworkError> {
        let links = if next {
            &self.next_crosswalk
        } else {
            &self.previous_crosswalk
        };
        let link = links
            .get(branch)
            .ok_or(super::network::RainNetworkError::PinIndex(branch as u32))?;
        let Some(intersection) = rain.intersection(link.id, mesh)? else {
            return Ok(Self::default());
        };
        intersection
            .ped
            .get(link.pin0 as usize)
            .copied()
            .ok_or(super::network::RainNetworkError::PinIndex(link.pin0))
    }

    fn target_list<'a>(
        &self,
        rain: &'a mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        end: bool,
    ) -> Result<Option<&'a mut super::nav_targets::NavTargetList>, super::network::RainNetworkError>
    {
        if self.kind != NodeType::Crosswalk {
            return Ok(None);
        }
        let Some(intersection) = rain.intersection(self.this_crosswalk.id, mesh)? else {
            return Ok(None);
        };
        let pin = if end {
            self.this_crosswalk.pin1
        } else {
            self.this_crosswalk.pin0
        };
        Ok(Some(
            intersection
                .nav_targets
                .get_mut(pin as usize)
                .ok_or(super::network::RainNetworkError::PinIndex(pin))?,
        ))
    }

    /// RequestStart/EndNavTarget82e53be0/53c80, retained intersection slots.
    pub(crate) fn request_nav_target<'a>(
        &self,
        rain: &'a mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        end: bool,
    ) -> Result<Option<&'a mut super::nav_targets::NavTarget>, super::network::RainNetworkError>
    {
        Ok(self
            .target_list(rain, mesh, end)?
            .and_then(|list| list.request()))
    }

    /// GetStart/EndNavTarget82e53d20/53db8 indexes the fixed 16-slot storage.
    pub(crate) fn nav_target<'a>(
        &self,
        rain: &'a mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        end: bool,
        id: u32,
    ) -> Result<Option<&'a mut super::nav_targets::NavTarget>, super::network::RainNetworkError>
    {
        let Some(list) = self.target_list(rain, mesh, end)? else {
            return Ok(None);
        };
        Ok(Some(list.targets.get_mut(id as usize).ok_or(
            super::network::RainNetworkError::NavTargetCapacity,
        )?))
    }

    /// FreeStart/EndNavTarget82e53e50/53ef8 clears occupancy AND entity owner.
    pub(crate) fn free_nav_target(
        &self,
        rain: &mut super::RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        end: bool,
        id: u32,
    ) -> Result<(), super::network::RainNetworkError> {
        if let Some(target) = self.nav_target(rain, mesh, end, id)? {
            target.occupied = false;
            target.entity_id = Some(0);
        }
        Ok(())
    }

    /// Road.PopulatePedData82e4dff8; constructor zeros untouched fields.
    pub(crate) fn road(id: u64, origin: &Pin, destination: &Pin) -> Self {
        let mut node = Self {
            kind: NodeType::Road,
            this_road: SubNode {
                id,
                pin0: origin.item_pin,
                pin1: destination.item_pin,
            },
            ..Self::default()
        };
        if origin.item_type == 1 {
            node.incoming_road[0].id = origin.item_id;
        } else if origin.item_type == 0 {
            node.previous_crosswalk[0] = SubNode {
                id: origin.item_id,
                pin0: origin.item_pin,
                pin1: 0,
            };
            node.previous_crosswalk[1] = SubNode {
                id: origin.item_id,
                pin0: origin.item_pin.wrapping_add(1) & 3,
                pin1: 0,
            };
        }
        if destination.item_type == 1 {
            node.outgoing_road[0].id = destination.item_id;
        } else if destination.item_type == 0 {
            node.next_crosswalk[0] = SubNode {
                id: destination.item_id,
                pin0: destination.item_pin,
                pin1: 0,
            };
            node.next_crosswalk[1] = SubNode {
                id: destination.item_id,
                pin0: destination.item_pin.wrapping_sub(1) & 3,
                pin1: 0,
            };
        }
        node
    }
    /// Intersection.PopulatePedData82e52c68, checked raw to disambiguate
    /// overlapping 64-bit register pairs and each 12-byte pin-array stride.
    pub(crate) fn intersection(id: u64, incoming: &[Pin; 4], outgoing: &[Pin; 4]) -> [Self; 4] {
        std::array::from_fn(|i| {
            let next = (i + 1) & 3;
            let previous = (i + 3) & 3;
            let from_pin = |pin: &Pin| SubNode {
                id: pin.item_id,
                pin0: pin.item_pin,
                pin1: 0,
            };
            Self {
                kind: NodeType::Crosswalk,
                this_crosswalk: SubNode {
                    id,
                    pin0: i as u32,
                    pin1: next as u32,
                },
                next_crosswalk: [
                    SubNode {
                        id,
                        pin0: next as u32,
                        pin1: ((next + 1) & 3) as u32,
                    },
                    SubNode::default(),
                ],
                previous_crosswalk: [
                    SubNode {
                        id,
                        pin0: previous as u32,
                        pin1: i as u32,
                    },
                    SubNode::default(),
                ],
                incoming_road: [from_pin(&incoming[i]), from_pin(&outgoing[previous])],
                outgoing_road: [from_pin(&outgoing[i]), from_pin(&incoming[next])],
                ..Self::default()
            }
        })
    }
}
