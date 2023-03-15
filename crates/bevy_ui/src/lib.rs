//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! This UI is laid out with the Flexbox paradigm (see <https://cssreference.io/flexbox/>)
mod flex;
mod focus;
mod geometry;
mod render;
mod stack;
mod ui_node;

#[cfg(feature = "bevy_text")]
mod accessibility;
pub mod camera_config;
pub mod node_bundles;
pub mod update;

#[cfg(feature = "bevy_text")]
use bevy_render::extract_component::ExtractComponentPlugin;
pub use flex::*;
pub use focus::*;
pub use geometry::*;
pub use render::*;
pub use ui_node::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera_config::*, geometry::*, node_bundles::*, ui_node::*, Interaction, UiScale,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::InputSystem;
use bevy_transform::TransformSystem;
use stack::ui_stack_system;
pub use stack::UiStack;
use update::update_clipping_system;

use crate::prelude::UiCameraConfig;

/// The basic plugin for Bevy UI
#[derive(Default)]
pub struct UiPlugin;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UiSystem {
    /// After this label, the ui flex state has been updated
    Flex,
    /// After this label, input interactions with UI entities have been updated for this frame
    Focus,
    /// After this label, the [`UiStack`] resource has been updated
    Stack,
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
            .init_resource::<UiStack>()
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
            .configure_set(UiSystem::Focus.in_base_set(CoreSet::PreUpdate))
            .configure_set(UiSystem::Flex.in_base_set(CoreSet::PostUpdate))
            .configure_set(UiSystem::Stack.in_base_set(CoreSet::PostUpdate))
            .add_system(ui_focus_system.in_set(UiSystem::Focus).after(InputSystem));
        // add these systems to front because these must run before transform update systems
        #[cfg(feature = "bevy_text")]
        app.add_plugin(accessibility::AccessibilityPlugin);
        app.add_systems((
            flex_node_system
                .in_set(UiSystem::Flex)
                .before(TransformSystem::TransformPropagate),
            ui_stack_system.in_set(UiSystem::Stack),
            update_clipping_system
                .after(TransformSystem::TransformPropagate)
                .in_base_set(CoreSet::PostUpdate),
        ));

        crate::render::build_ui_render(app);
    }
}
