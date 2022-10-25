//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! # Basic usage
//! Spawn UI elements with [`entity::ButtonBundle`], [`entity::ImageBundle`], [`entity::TextBundle`] and [`entity::NodeBundle`]
//! This UI is laid out with the Flexbox paradigm (see <https://cssreference.io/flexbox/>)
mod flex;
mod focus;
mod geometry;
mod render;
mod ui_node;

pub mod entity;
pub mod update;
pub mod widget;

use bevy_render::extract_component::ExtractComponentPlugin;
pub use flex::*;
pub use focus::*;
pub use geometry::*;
pub use render::*;
pub use ui_node::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{entity::*, geometry::*, ui_node::*, widget::Button, Interaction, UiScale};
}

use bevy_app::prelude::*;
use bevy_ecs::{
    schedule::{IntoSystemDescriptor, SystemLabel},
    system::Resource,
};
use bevy_input::InputSystem;
use bevy_transform::TransformSystem;
use bevy_window::ModifiesWindows;
use update::{ui_z_system, update_clipping_system};

use crate::prelude::UiCameraConfig;

/// The basic plugin for Bevy UI
#[derive(Default)]
pub struct UiPlugin;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiSystem {
    /// After this label, the ui flex state has been updated
    Flex,
    /// After this label, input interactions with UI entities have been updated for this frame
    Focus,
}

/// The current scale of the UI.
///
/// A multiplier to fixed-sized ui values.
/// **Note:** This will only affect fixed ui values like [`Val::Px`]
#[derive(Debug, Resource)]
pub struct UiScale {
    /// The scale to be applied.
    pub scale: f64,
}

impl Default for UiScale {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<UiCameraConfig>::default())
            .init_resource::<FlexSurface>()
            .init_resource::<UiScale>()
            .register_type::<AlignContent>()
            .register_type::<AlignItems>()
            .register_type::<AlignSelf>()
            .register_type::<CalculatedSize>()
            .register_type::<Direction>()
            .register_type::<Display>()
            .register_type::<FlexDirection>()
            .register_type::<FlexWrap>()
            .register_type::<FocusPolicy>()
            .register_type::<Interaction>()
            .register_type::<JustifyContent>()
            .register_type::<Node>()
            // NOTE: used by Style::aspect_ratio
            .register_type::<Option<f32>>()
            .register_type::<Overflow>()
            .register_type::<PositionType>()
            .register_type::<Size>()
            .register_type::<UiRect>()
            .register_type::<Style>()
            .register_type::<BackgroundColor>()
            .register_type::<UiImage>()
            .register_type::<Val>()
            .register_type::<widget::Button>()
            .register_type::<widget::Slider>()
            .register_type::<widget::ImageMode>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                ui_focus_system.label(UiSystem::Focus).after(InputSystem),
            )
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::text_system
                    .before(UiSystem::Flex)
                    .after(ModifiesWindows),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::image_node_system.before(UiSystem::Flex),
            )
            .add_system_to_stage(
                CoreStage::Update,
                widget::update_slider_value.label(widget::UpdateSliderValue),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::update_slider_handle.after(widget::UpdateSliderValue),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                flex_node_system
                    .label(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate)
                    .after(ModifiesWindows),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                ui_z_system
                    .after(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_clipping_system.after(TransformSystem::TransformPropagate),
            );

        crate::render::build_ui_render(app);
    }
}
