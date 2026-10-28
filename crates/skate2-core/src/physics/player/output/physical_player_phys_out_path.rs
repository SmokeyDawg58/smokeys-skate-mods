//! Path-recording constructor82e2a6b0 and its three actual Node constructors.

/// No extended node is produced by the recovered constructor. Its later
/// producers must recover the payload before this pointer can become non-NULL.
#[derive(Debug, PartialEq)]
pub(super) enum ExtendedNodeData {}

#[derive(Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(super) struct Node {
    pub(super) position: [f32; 3],
    pub(super) direction: [f32; 3],
    pub(super) board_orientation: [u8; 4],
    pub(super) skater_orientation: [u8; 4],
    pub(super) extended: Option<Box<ExtendedNodeData>>,
    pub(super) frames_since_last_node: u8,
    pub(super) width_left: u8,
    pub(super) width_right: u8,
    pub(super) event_type: u8,
    pub(super) flags: u8,
    pub(super) extra: [u8; 3],
}
impl Node {
    /// Node824b6538 writes quaternion bytes0,0,0,255 before the later Reset.
    pub(super) fn new() -> Self {
        Self {
            position: [0.0; 3],
            direction: [0.0; 3],
            board_orientation: [0, 0, 0, 255],
            skater_orientation: [0, 0, 0, 255],
            extended: None,
            frames_since_last_node: 0,
            width_left: 0,
            width_right: 0,
            event_type: 0,
            flags: 0,
            extra: [0; 3],
        }
    }
    /// Reset824c7d20 packs ZERO quaternion, not the constructor quaternion.
    /// `PackComponent824b2588` evaluates (0+1)*127.5 and truncates to127.
    /// `ExtraData1`..3 are deliberately preserved by Reset.
    pub(super) fn reset(&mut self) {
        self.position = [0.0; 3];
        self.direction = [0.0; 3];
        self.board_orientation = [127; 4];
        self.skater_orientation = [127; 4];
        self.extended = None;
        self.frames_since_last_node = 0;
        self.width_left = 0;
        self.width_right = 0;
        self.event_type = 0;
        self.flags = 0;
    }
}

#[derive(Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct PathRecording {
    pub(super) dummy: i32,
    pub(super) new_node: Node,
    pub(super) updated_node: Node,
    pub(super) potential_end_node: Node,
    pub(super) has_new_trajectory: bool,
    pub(super) has_new_node: bool,
    pub(super) has_updated_node: bool,
    pub(super) has_potential_end_node: bool,
}
impl PathRecording {
    pub(super) fn new() -> Self {
        let mut result = Self {
            dummy: 0,
            new_node: Node::new(),
            updated_node: Node::new(),
            potential_end_node: Node::new(),
            has_new_trajectory: false,
            has_new_node: false,
            has_updated_node: false,
            has_potential_end_node: false,
        };
        result.reset();
        result
    }
    pub(super) fn reset(&mut self) {
        self.dummy = 0;
        self.has_new_trajectory = false;
        self.has_new_node = false;
        self.has_updated_node = false;
        self.has_potential_end_node = false;
        self.new_node.reset();
        self.updated_node.reset();
        self.potential_end_node.reset();
    }
}
