/*!
 * SPDX-License-Identifier: MIT
 *
 * Based on code from smooth-bevy-cameras.
 */

use smooth_bevy_cameras::{LookAngles, LookTransform, LookTransformBundle, Smoother};

use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    input::{
        mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
        prelude::*,
    },
    math::prelude::*,
    render::prelude::*,
    transform::components::Transform,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Plugin {
    pub override_input_system: bool,
}

impl Plugin {
    pub fn new(override_input_system: bool) -> Self {
        Self {
            override_input_system,
        }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut AppBuilder) {
        let app = app
            .add_system(control_system.system())
            .add_event::<ControlEvent>();
        if !self.override_input_system {
            app.add_system(mouse_input_map.system());
        }
    }
}

#[derive(Bundle)]
pub struct CameraBundle {
    controller: Controller,
    #[bundle]
    look_transform: LookTransformBundle,
    #[bundle]
    perspective: PerspectiveCameraBundle,
}

impl CameraBundle {
    pub fn new(
        controller: Controller,
        mut perspective: PerspectiveCameraBundle,
        eye: Vec3,
        target: Vec3,
    ) -> Self {
        // Make sure the transform is consistent with the controller to start.
        perspective.transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);

        Self {
            controller,
            look_transform: LookTransformBundle {
                transform: LookTransform { eye, target },
                smoother: Smoother::new(controller.smoothing_weight),
            },
            perspective,
        }
    }
}

/// A camera similar to that from RTS games:
/// floats at a set horizontal height,
/// can move vertically,
/// can rotate in place.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Controller {
    pub enabled: bool,
    pub mouse_rotate_sensitivity: Vec2,
    pub mouse_translate_sensitivity: Vec2,
    pub trackpad_translate_sensitivity: f32,
    pub wheel_translate_sensitivity: f32,
    pub smoothing_weight: f32,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            enabled: true,
            mouse_rotate_sensitivity: Vec2::splat(0.002),
            mouse_translate_sensitivity: Vec2::splat(0.1),
            trackpad_translate_sensitivity: 0.1,
            wheel_translate_sensitivity: 1.0,
            smoothing_weight: 0.9,
        }
    }
}

pub enum ControlEvent {
    /// Pan across the horizontal axes
    Pan(Vec2),
    /// Pitch and yaw
    Rotate(Vec2),
    /// Vertical translation
    TranslateVertical(f32),
}

pub fn mouse_input_map(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    controllers: Query<&Controller>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().next() {
        controller
    } else {
        return;
    };
    let Controller {
        enabled,
        mouse_translate_sensitivity,
        mouse_rotate_sensitivity,
        wheel_translate_sensitivity,
        ..
    } = *controller;

    if !enabled {
        return;
    }

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += event.delta;
    }

    match (
        mouse_buttons.pressed(MouseButton::Middle),
        mouse_buttons.pressed(MouseButton::Right),
    ) {
        (false, true) => {
            events.send(ControlEvent::Pan(
                mouse_translate_sensitivity * cursor_delta,
            ));
        }
        (true, false) => {
            events.send(ControlEvent::Rotate(
                mouse_rotate_sensitivity * cursor_delta,
            ));
        }
        _ => (),
    }

    let vertical_delta: f32 = mouse_wheel_reader.iter()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 5.0,
        })
        .sum();
    events.send(ControlEvent::TranslateVertical(
        wheel_translate_sensitivity * vertical_delta,
    ));
}

pub fn control_system(
    mut events: EventReader<ControlEvent>,
    mut cameras: Query<(&Controller, &mut LookTransform)>,
) {
    // Can only control one camera at a time.
    let (controller, mut transform) =
        if let Some((controller, transform)) = cameras.iter_mut().next() {
            (controller, transform)
        } else {
            return;
        };

    if controller.enabled {
        let look_vector = transform.look_direction().unwrap();
        let mut look_angles = LookAngles::from_vector(look_vector);
        let forward_vector = Vec3::new(look_vector.x, 0.0, look_vector.z).normalize();
        let side_vector = Quat::from_rotation_y(std::f32::consts::TAU / 4.0)
            .mul_vec3(forward_vector);

        let yaw_rot = Quat::from_axis_angle(Vec3::Y, look_angles.get_yaw());

        for event in events.iter() {
            match event {
                ControlEvent::Pan(delta) => {
                    // Translates forward/backward and to the side
                    transform.eye += delta.y * forward_vector;
                    transform.eye += delta.x * side_vector;
                }
                ControlEvent::Rotate(delta) => {
                    // Rotates with pitch and yaw.
                    look_angles.add_yaw(-delta.x);
                    look_angles.add_pitch(-delta.y);
                }
                ControlEvent::TranslateVertical(delta) => {
                    // Translates up/down (Y)
                    transform.eye += Vec3::new(0.0, *delta, 0.0);
                }
            }
        }

        look_angles.assert_not_looking_up();

        transform.target = transform.eye + transform.radius() * look_angles.unit_vector();
    } else {
        events.iter(); // Drop the events.
    }
}
