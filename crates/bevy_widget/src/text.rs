use bevy_app::{App, CoreSet, Plugin};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    prelude::Bundle,
    query::{Changed, Or, With},
    schedule::IntoSystemConfig,
    system::{Commands, Local, ParamSet, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_render::{
    camera::CameraUpdateSystem,
    texture::Image,
    view::{ComputedVisibility, Visibility},
};
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextAlignment, TextError, TextLayoutInfo,
    TextPipeline, TextSection, TextSettings, TextStyle, YAxisOrientation,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui::{CalculatedSize, FocusPolicy, Node, Style, UiScale, UiSystem, Val, ZIndex};
use bevy_window::{PrimaryWindow, Window};

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

/// Defines how `min_size`, `size`, and `max_size` affects the bounds of a text
/// block.
pub fn text_constraint(min_size: Val, size: Val, max_size: Val, scale_factor: f64) -> f32 {
    // Needs support for percentages
    match (min_size, size, max_size) {
        (_, _, Val::Px(max)) => scale_value(max, scale_factor),
        (Val::Px(min), _, _) => scale_value(min, scale_factor),
        (Val::Auto, Val::Px(size), Val::Auto) => scale_value(size, scale_factor),
        _ => f32::MAX,
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut commands: Commands,
    mut queued_text_ids: Local<Vec<Entity>>,
    mut last_scale_factor: Local<f64>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Or<(Changed<Text>, Changed<Node>, Changed<Style>)>>,
        Query<Entity, (With<Text>, With<Style>)>,
        Query<(
            &Text,
            &Style,
            &mut CalculatedSize,
            Option<&mut TextLayoutInfo>,
        )>,
    )>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    let inv_scale_factor = 1. / scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            queued_text_ids.push(entity);
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text_ids.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text_ids.is_empty() {
        return;
    }

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text_ids.drain(..) {
        if let Ok((text, style, mut calculated_size, text_layout_info)) = query.get_mut(entity) {
            let node_size = Vec2::new(
                text_constraint(
                    style.min_size.width,
                    style.size.width,
                    style.max_size.width,
                    scale_factor,
                ),
                text_constraint(
                    style.min_size.height,
                    style.size.height,
                    style.max_size.height,
                    scale_factor,
                ),
            );

            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behaviour,
                node_size,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                &mut font_atlas_warning,
                YAxisOrientation::TopToBottom,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    calculated_size.size = Vec2::new(
                        scale_value(info.size.x, inv_scale_factor),
                        scale_value(info.size.y, inv_scale_factor),
                    );
                    match text_layout_info {
                        Some(mut t) => *t = info,
                        None => {
                            commands.entity(entity).insert(info);
                        }
                    }
                }
            }
        }
    }

    *queued_text_ids = new_queue;
}

/// A plugin for adding text rendering
#[derive(Default)]
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            text_system
                .in_base_set(CoreSet::PostUpdate)
                .before(UiSystem::Flex)
                // Potential conflict: `Assets<Image>`
                // In practice, they run independently since `bevy_render::camera_update_system`
                // will only ever observe its own render target, and `widget::text_system`
                // will never modify a pre-existing `Image` asset.
                .ambiguous_with(CameraUpdateSystem)
                // Potential conflict: `Assets<Image>`
                // Since both systems will only ever insert new [`Image`] assets,
                // they will never observe each other's effects.
                .ambiguous_with(bevy_text::update_text2d_layout),
        );
    }
}

/// A UI node that is text
#[derive(Bundle, Clone, Debug, Default)]
pub struct TextBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl TextBundle {
    /// Create a [`TextBundle`] from a single section.
    ///
    /// See [`Text::from_section`] for usage.
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: Text::from_section(value, style),
            ..Default::default()
        }
    }

    /// Create a [`TextBundle`] from a list of sections.
    ///
    /// See [`Text::from_sections`] for usage.
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            text: Text::from_sections(sections),
            ..Default::default()
        }
    }

    /// Returns this [`TextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub const fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text.alignment = alignment;
        self
    }

    /// Returns this [`TextBundle`] with a new [`Style`].
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}
