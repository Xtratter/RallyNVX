//! What the ground is made of at a point (grip, dust and sound will read it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface { Grass, Dirt, Sand, Rock, Gravel, Asphalt, Water }
