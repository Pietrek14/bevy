#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! The official collection of user interface widgets for `bevy_ui`.

mod button;
mod image;
mod text;

pub use button::*;
pub use image::*;
pub use text::*;

use bevy_app::{App, Plugin};

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use super::{Button, ButtonBundle, ImageBundle, TextBundle};
}

/// The plugin for UI widgets
#[derive(Default)]
pub struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ButtonPlugin)
            .add_plugin(TextPlugin)
            .add_plugin(ImagePlugin);
    }
}