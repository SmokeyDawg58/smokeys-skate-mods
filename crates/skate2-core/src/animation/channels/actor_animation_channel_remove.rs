//! Ordinary `ChannelAnimatable::RemoveChannel82d7d8a8` on actual owners.
use super::{
    ActorAnimationConstruction, AnimationPointerState,
    channel_runtime::{ChannelBaseReference, GestureChannelError as E},
};
use crate::play_animation::FastString36;

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Disposable original-asset Actor only, after all channel/body corpus checks.
    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one disposable owner traverses all removal positions and verifies native stale payloads"
    )]
    pub(super) fn check_remove_channels_corpus(&mut self) {
        let missing = FastString36::encode("NO_SUCH_CHANNEL_REMOVE").unwrap();
        let channels = self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap();
        let before = (
            channels.free_channels.clone(),
            channels.free_trees.clone(),
            channels.root,
        );
        self.try_remove_channel(&missing).unwrap();
        let channels = self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap();
        assert_eq!(
            (
                channels.free_channels.clone(),
                channels.free_trees.clone(),
                channels.root
            ),
            before
        );
        assert!(channels.dictionary.len() >= 3);
        // Existing real owner provides >=3 nodes. Select middle, then bottom,
        // then repeatedly root; final iteration exercises the single-node case.
        let mut step = 0;
        loop {
            let channels = self
                .initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap();
            let Some(root) = channels.root else {
                break;
            };
            let index = if step == 0 {
                channels.channels[root].base.unwrap()
            } else if step == 1 {
                let mut index = root;
                while let Some(base) = channels.channels[index].base {
                    index = base;
                }
                index
            } else {
                root
            };
            let name = *channels
                .dictionary
                .iter()
                .find(|(_, value)| **value == index)
                .unwrap()
                .0;
            let removed = &channels.channels[index];
            let parent = removed.parent;
            let base = removed.base;
            let tree = removed.channel_tree.unwrap();
            let animation = removed.runtime.as_ref().unwrap().animation.clone();
            let flags = removed.channel_flags;
            let time = removed.blend_in_time;
            let free = (channels.free_channels.len(), channels.free_trees.len());
            self.try_remove_channel(&name).unwrap();
            let channels = self
                .initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap();
            assert!(!channels.dictionary.contains_key(&name));
            assert_eq!(channels.free_channels.len(), free.0 + 1);
            assert_eq!(channels.free_channels.last(), Some(&index));
            assert_eq!(channels.free_trees.len(), free.1 + 1);
            assert_eq!(channels.free_trees.last(), Some(&tree));
            assert!(channels.trees[tree].runtime.node(&animation).is_err());
            let removed = &channels.channels[index];
            assert_eq!(removed.runtime.as_ref().unwrap().animation, animation);
            assert_eq!(removed.channel_flags, flags);
            assert_eq!(removed.blend_in_time, time);
            assert_eq!(
                (
                    removed.channel_tree,
                    removed.transition_tree,
                    removed.parent,
                    removed.base
                ),
                (None, None, None, None)
            );
            if let Some(parent) = parent {
                assert_eq!(channels.channels[parent].base, base);
                if let Some(base) = base {
                    assert_eq!(channels.channels[base].parent, Some(parent));
                    assert!(
                        matches!(channels.channels[parent].runtime.as_ref().unwrap().base,
                        ChannelBaseReference::Channel(actual) if actual == base)
                    );
                } else {
                    assert!(
                        matches!(&channels.channels[parent].runtime.as_ref().unwrap().base,
                        ChannelBaseReference::Body(actual) if Some(actual) == self.base.as_ref().unwrap().body_root.as_ref())
                    );
                }
            } else {
                assert_eq!(channels.root, base);
                if let Some(base) = base {
                    assert_eq!(channels.channels[base].parent, None);
                }
            }
            step += 1;
        }
        let channels = self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap();
        assert!(channels.dictionary.is_empty());
        assert_eq!(channels.free_channels.len(), channels.channels.len());
        assert_eq!(channels.free_trees.len(), channels.trees.len());
        assert_eq!(
            self.channel.as_ref().unwrap().root_channel,
            AnimationPointerState::Null
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "native release prefixes, dictionary erase and ordered link stores"
    )]
    pub(crate) fn try_remove_channel(&mut self, name: &FastString36) -> Result<(), E> {
        let channels = self
            .initialization
            .as_mut()
            .and_then(|state| state.channels.as_mut())
            .ok_or(E::MissingOwner("channels"))?;
        let Some(&index) = channels.dictionary.get(name) else {
            return Ok(());
        };
        // d914 publication precedes d94c count assertion and channel dereference.
        channels.free_channels.push(index);
        let maximum = self
            .channel
            .as_ref()
            .and_then(|channel| channel.max_channels)
            .and_then(|maximum| usize::try_from(maximum).ok())
            .ok_or(E::MissingOwner("channel maximum"))?;
        if channels.free_channels.len() > maximum {
            return Err(E::ChannelCapacity);
        }
        let tree = channels
            .channels
            .get(index)
            .ok_or(E::InvalidChannel)?
            .channel_tree
            .ok_or(E::MissingOwner("channel tree"))?;
        channels
            .trees
            .get_mut(tree)
            .ok_or(E::InvalidChannel)?
            .runtime
            .clear();
        channels.free_trees.push(tree);
        let tree_maximum = maximum.checked_mul(2).ok_or(E::ChannelCapacity)?;
        if channels.free_trees.len() > tree_maximum {
            return Err(E::ChannelCapacity);
        }
        if channels
            .channels
            .get(index)
            .ok_or(E::InvalidChannel)?
            .transitioning
        {
            let transition_tree = channels.channels[index]
                .transition_tree
                .ok_or(E::MissingOwner("transition tree"))?;
            channels
                .trees
                .get_mut(transition_tree)
                .ok_or(E::InvalidChannel)?
                .runtime
                .clear();
            channels.free_trees.push(transition_tree);
            if channels.free_trees.len() > tree_maximum {
                return Err(E::ChannelCapacity);
            }
        }
        // da7c: the native dictionary's remove must preserve bucket chain order.
        channels.dictionary.remove(name);
        let removed = channels.channels.get(index).ok_or(E::InvalidChannel)?;
        let base = removed.base; // da80 before parent da88
        let parent = removed.parent;
        if let Some(parent) = parent {
            if let Some(base) = base {
                channels
                    .channels
                    .get_mut(parent)
                    .ok_or(E::InvalidChannel)?
                    .base = Some(base);
                channels
                    .channels
                    .get_mut(base)
                    .ok_or(E::InvalidChannel)?
                    .parent = Some(parent);
                let parent = channels.channels.get_mut(parent).ok_or(E::InvalidChannel)?;
                parent
                    .runtime
                    .as_mut()
                    .ok_or(E::MissingOwner("parent channel runtime"))?
                    .base = ChannelBaseReference::Channel(base);
                parent.base_animation = AnimationPointerState::Constructed;
            } else {
                channels
                    .channels
                    .get_mut(parent)
                    .ok_or(E::InvalidChannel)?
                    .base = None;
                // dab8 precedes actual GetBaseAnim; do not use removed stale base.
                let body = self.base.as_ref().ok_or(E::MissingOwner("Animatable"))?;
                let (reference, mirror) = if let Some(root) = &body.body_root {
                    (
                        ChannelBaseReference::Body(root.clone()),
                        AnimationPointerState::Constructed,
                    )
                } else if body.base_animation == AnimationPointerState::Null {
                    (ChannelBaseReference::Empty, AnimationPointerState::Null)
                } else {
                    return Err(E::MissingOwner("body root"));
                };
                let parent = channels.channels.get_mut(parent).ok_or(E::InvalidChannel)?;
                parent
                    .runtime
                    .as_mut()
                    .ok_or(E::MissingOwner("parent channel runtime"))?
                    .base = reference;
                parent.base_animation = mirror;
            }
        } else if let Some(base) = base {
            channels
                .channels
                .get_mut(base)
                .ok_or(E::InvalidChannel)?
                .parent = None;
            channels.root = Some(base);
            self.channel
                .as_mut()
                .ok_or(E::MissingOwner("ChannelAnimatable"))?
                .root_channel = AnimationPointerState::Constructed;
        } else {
            channels.root = None;
            self.channel
                .as_mut()
                .ok_or(E::MissingOwner("ChannelAnimatable"))?
                .root_channel = AnimationPointerState::Null;
        }
        let removed = channels.channels.get_mut(index).ok_or(E::InvalidChannel)?;
        removed.channel_tree = None;
        removed.transition_tree = None;
        removed.transitioning = false;
        removed.parent = None;
        removed.base = None;
        removed.priority = 0;
        // Node runtime, clocks, flags, evaluated and inline payloads stay stale.
        Ok(())
    }
}
