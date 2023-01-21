use crate::{camera_config::UiCameraConfig, CalculatedClip, Node, UiStack};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    prelude::Component,
    query::WorldQuery,
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::camera::{Camera, RenderTarget};
use bevy_render::view::ComputedVisibility;
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Describes what type of input interaction has occurred for a UI node.
///
/// This is commonly queried with a `Changed<Interaction>` filter.
///
/// Updated in [`ui_focus_system`].
///
/// If a UI node has both [`Interaction`] and [`ComputedVisibility`] components,
/// [`Interaction`] will always be [`Interaction::None`]
/// when [`ComputedVisibility::is_visible()`] is false.
/// This ensures that hidden UI nodes are not interactable,
/// and do not end up stuck in an active state if hidden at the wrong time.
///
/// Note that you can also control the visibility of a node using the [`Display`](crate::ui_node::Display) property,
/// which fully collapses it during layout calculations.
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum Interaction {
    /// The node has been clicked
    Clicked,
    /// The node has been hovered over
    Hovered,
    /// Nothing has happened
    None,
}

impl Interaction {
    const DEFAULT: Self = Self::None;
}

impl Default for Interaction {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Describes whether the [`Interaction`] component should remain in the `Clicked` state after
/// the cursor stops hovering over the node.
///
/// When the user clicks this node and the `InteractionPolicy` is set to `Hold`, the `Interaction` component will remain in the `Clicked` state after the cursor leaves the node, until the user releases the interaction button. (the default behaviour)
///
/// If instead the `InteractionPolicy` is set to `Release`, the `Interaction` component will be set to `None`
/// as soon as the cursor leaves the node, even if the user still is pressing down the interaction button.
#[derive(
    Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum InteractionPolicy {
    /// Keep the node clicked after it stops being hovered
    #[default]
    Hold,
    /// Release the node if the cursor stops hovering
    Release,
}

/// A component storing the position of the mouse relative to the node, (0., 0.) being the top-left corner and (1., 1.) being the bottom-right
/// If the mouse is not over the node, the value will go beyond the range of (0., 0.) to (1., 1.)
/// A None value means that the cursor position is unknown.
///
/// It can be used alongside interaction to get the position of the press.
#[derive(
    Component,
    Deref,
    DerefMut,
    Copy,
    Clone,
    Default,
    PartialEq,
    Debug,
    Reflect,
    Serialize,
    Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub struct RelativeCursorPosition {
    /// Cursor position relative to size and position of the Node.
    pub normalized: Option<Vec2>,
}

impl RelativeCursorPosition {
    /// A helper function to check if the mouse is over the node
    pub fn mouse_over(&self) -> bool {
        self.normalized
            .map(|position| (0.0..1.).contains(&position.x) && (0.0..1.).contains(&position.y))
            .unwrap_or(false)
    }
}

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum FocusPolicy {
    /// Blocks interaction
    Block,
    /// Lets interaction pass through
    Pass,
}

impl FocusPolicy {
    const DEFAULT: Self = Self::Pass;
}

impl Default for FocusPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Contains entities whose Interaction should be set to None
#[derive(Default)]
pub struct State {
    entities_to_reset: SmallVec<[Entity; 1]>,
}

/// Main query for [`ui_focus_system`]
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NodeQuery {
    entity: Entity,
    node: &'static Node,
    global_transform: &'static GlobalTransform,
    interaction: Option<&'static mut Interaction>,
    interaction_policy: Option<&'static InteractionPolicy>,
    relative_cursor_position: Option<&'static mut RelativeCursorPosition>,
    focus_policy: Option<&'static FocusPolicy>,
    calculated_clip: Option<&'static CalculatedClip>,
    computed_visibility: Option<&'static ComputedVisibility>,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
///
/// Entities with a hidden [`ComputedVisibility`] are always treated as released.
pub fn ui_focus_system(
    mut state: Local<State>,
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    ui_stack: Res<UiStack>,
    mut node_query: Query<NodeQuery>,
) {
    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut interaction) = node_query.get_component_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();
    if mouse_released {
        for node in node_query.iter_mut() {
            if let Some(mut interaction) = node.interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.any_just_pressed();

    let is_ui_disabled =
        |camera_ui| matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. }));

    let cursor_position = camera
        .iter()
        .filter(|(_, camera_ui)| !is_ui_disabled(*camera_ui))
        .filter_map(|(camera, _)| {
            if let RenderTarget::Window(window_id) = camera.target {
                Some(window_id)
            } else {
                None
            }
        })
        .filter_map(|window_id| windows.get(window_id))
        .filter(|window| window.is_focused())
        .find_map(|window| {
            window.cursor_position().map(|mut cursor_pos| {
                cursor_pos.y = window.height() - cursor_pos.y;
                cursor_pos
            })
        })
        .or_else(|| touches_input.first_pressed_position());

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.
    let mut moused_over_nodes = ui_stack
        .uinodes
        .iter()
        // reverse the iterator to traverse the tree from closest nodes to furthest
        .rev()
        .filter_map(|entity| {
            if let Ok(node) = node_query.get_mut(*entity) {
                // Nodes that are not rendered should not be interactable
                if let Some(computed_visibility) = node.computed_visibility {
                    if !computed_visibility.is_visible() {
                        // Reset their interaction to None to avoid strange stuck state
                        if let Some(mut interaction) = node.interaction {
                            // We cannot simply set the interaction to None, as that will trigger change detection repeatedly
                            interaction.set_if_neq(Interaction::None);
                        }

                        return None;
                    }
                }

                let position = node.global_transform.translation();
                let ui_position = position.truncate();
                let extents = node.node.size() / 2.0;
                let mut min = ui_position - extents;
                if let Some(clip) = node.calculated_clip {
                    min = Vec2::max(min, clip.clip.min);
                }

                // The mouse position relative to the node
                // (0., 0.) is the top-left corner, (1., 1.) is the bottom-right corner
                let relative_cursor_position = cursor_position.map(|cursor_position| {
                    Vec2::new(
                        (cursor_position.x - min.x) / node.node.size().x,
                        (cursor_position.y - min.y) / node.node.size().y,
                    )
                });

                // If the current cursor position is within the bounds of the node, consider it for
                // clicking
                let relative_cursor_position_component = RelativeCursorPosition {
                    normalized: relative_cursor_position,
                };

                let contains_cursor = relative_cursor_position_component.mouse_over();

                // Save the relative cursor position to the correct component
                if let Some(mut node_relative_cursor_position_component) =
                    node.relative_cursor_position
                {
                    *node_relative_cursor_position_component = relative_cursor_position_component;
                }

                if contains_cursor {
                    Some(*entity)
                } else {
                    if let Some(mut interaction) = node.interaction {
                        // If the InteractionPolicy is Release, we should set the interaction to None
                        // The entity might just as well not have the InteractionPolicy component
                        // in which case we should use the default behaviour
                        let interaction_policy =
                            node.interaction_policy.copied().unwrap_or_default();

                        if *interaction == Interaction::Hovered
                            || (cursor_position.is_none())
                            || interaction_policy == InteractionPolicy::Release
                        {
                            interaction.set_if_neq(Interaction::None);
                        }
                    }
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<Entity>>()
        .into_iter();

    // set Clicked or Hovered on top nodes. as soon as a node with a `Block` focus policy is detected,
    // the iteration will stop on it because it "captures" the interaction.
    let mut iter = node_query.iter_many_mut(moused_over_nodes.by_ref());
    while let Some(node) = iter.fetch_next() {
        if let Some(mut interaction) = node.interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "clickable"
                if *interaction != Interaction::Clicked {
                    *interaction = Interaction::Clicked;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(node.entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
            }
        }

        match node.focus_policy.unwrap_or(&FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
        }
    }
    // reset `Interaction` for the remaining lower nodes to `None`. those are the nodes that remain in
    // `moused_over_nodes` after the previous loop is exited.
    let mut iter = node_query.iter_many_mut(moused_over_nodes);
    while let Some(node) = iter.fetch_next() {
        if let Some(mut interaction) = node.interaction {
            // don't reset clicked nodes because they're handled separately
            if *interaction != Interaction::Clicked {
                interaction.set_if_neq(Interaction::None);
            }
        }
    }
}
