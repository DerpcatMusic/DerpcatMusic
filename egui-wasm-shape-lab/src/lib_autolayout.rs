#![allow(clippy::too_many_arguments)]
#![allow(clippy::missing_safety_doc)]

mod geometry;

use core::ptr;
use egui::{
    Color32, Context, Event, Id, Mesh, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Sense, Shape, Stroke, Ui, Vec2, pos2, vec2,
};
use geometry::{MergeStats, merged_rounded_loops, rectangle};

const MAGIC: u32 = 0x4547_5549; // "EGUI"
const VERTEX_STRIDE: usize = 6; // x, y, premultiplied RGBA as floats

include!("design.inc.rs");
include!("app.inc.rs");
include!("helpers.inc.rs");
