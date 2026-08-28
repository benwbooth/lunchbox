#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, available)]
        #[qproperty(i32, connected_count)]
        #[qproperty(QString, active_device)]
        #[qproperty(QString, controller_layout)]
        #[qproperty(QString, status_message)]
        #[qproperty(QString, navigation_action)]
        #[qproperty(i32, navigation_revision)]
        type GamepadInput = super::GamepadInputRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut GamepadInput>);

        #[qinvokable]
        fn button_label(self: &GamepadInput, action: QString) -> QString;

        #[qinvokable]
        fn probe_navigation_action(self: Pin<&mut GamepadInput>, action: QString);
    }

    impl cxx_qt::Threading for GamepadInput {}
}

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs, GilrsBuilder};

const AXIS_ENGAGE: f32 = 0.68;
const AXIS_RELEASE: f32 = 0.38;
const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(380);
const REPEAT_INTERVAL: Duration = Duration::from_millis(90);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(16);

pub struct GamepadInputRust {
    initialized: bool,
    ready: bool,
    available: bool,
    connected_count: i32,
    active_device: QString,
    controller_layout: QString,
    status_message: QString,
    navigation_action: QString,
    navigation_revision: i32,
    stop: Arc<AtomicBool>,
}

impl Default for GamepadInputRust {
    fn default() -> Self {
        Self {
            initialized: false,
            ready: false,
            available: false,
            connected_count: 0,
            active_device: QString::default(),
            controller_layout: QString::from("xbox"),
            status_message: QString::from("Gamepad input starts with Couch Mode."),
            navigation_action: QString::default(),
            navigation_revision: 0,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for GamepadInputRust {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum NavigationAction {
    Up,
    Down,
    Left,
    Right,
    PageLeft,
    PageRight,
    Accept,
    Back,
    Favorite,
    Details,
    Menu,
    Home,
}

impl NavigationAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::PageLeft => "page_left",
            Self::PageRight => "page_right",
            Self::Accept => "accept",
            Self::Back => "back",
            Self::Favorite => "favorite",
            Self::Details => "details",
            Self::Menu => "menu",
            Self::Home => "home",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "page_left" => Some(Self::PageLeft),
            "page_right" => Some(Self::PageRight),
            "accept" => Some(Self::Accept),
            "back" => Some(Self::Back),
            "favorite" => Some(Self::Favorite),
            "details" => Some(Self::Details),
            "menu" => Some(Self::Menu),
            "home" => Some(Self::Home),
            _ => None,
        }
    }

    fn repeats(self) -> bool {
        matches!(
            self,
            Self::Up | Self::Down | Self::Left | Self::Right | Self::PageLeft | Self::PageRight
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RepeatSource {
    Button(Button),
    HorizontalAxis,
    VerticalAxis,
}

#[derive(Clone, Copy, Debug)]
struct HeldNavigation {
    action: NavigationAction,
    next_repeat: Instant,
}

#[derive(Default)]
struct NavigationTranslator {
    held: HashMap<RepeatSource, HeldNavigation>,
    horizontal_axis: i8,
    vertical_axis: i8,
}

impl NavigationTranslator {
    fn handle_event(&mut self, event: EventType, now: Instant) -> Option<NavigationAction> {
        match event {
            EventType::ButtonPressed(button, _) => {
                let action = button_action(button)?;
                if action.repeats() {
                    self.press(RepeatSource::Button(button), action, now)
                } else {
                    Some(action)
                }
            }
            EventType::ButtonRepeated(button, _) => {
                button_action(button).filter(|action| action.repeats())
            }
            EventType::ButtonReleased(button, _) => {
                self.held.remove(&RepeatSource::Button(button));
                None
            }
            EventType::AxisChanged(axis, value, _) => self.update_axis(axis, value, now),
            EventType::Disconnected => {
                self.clear();
                None
            }
            _ => None,
        }
    }

    fn press(
        &mut self,
        source: RepeatSource,
        action: NavigationAction,
        now: Instant,
    ) -> Option<NavigationAction> {
        if self
            .held
            .get(&source)
            .is_some_and(|held| held.action == action)
        {
            return None;
        }
        self.held.insert(
            source,
            HeldNavigation {
                action,
                next_repeat: now + INITIAL_REPEAT_DELAY,
            },
        );
        Some(action)
    }

    fn update_axis(&mut self, axis: Axis, value: f32, now: Instant) -> Option<NavigationAction> {
        let (source, current, negative, positive) = match axis {
            Axis::LeftStickX | Axis::DPadX => (
                RepeatSource::HorizontalAxis,
                &mut self.horizontal_axis,
                NavigationAction::Left,
                NavigationAction::Right,
            ),
            Axis::LeftStickY | Axis::DPadY => (
                RepeatSource::VerticalAxis,
                &mut self.vertical_axis,
                NavigationAction::Down,
                NavigationAction::Up,
            ),
            _ => return None,
        };
        let direction = axis_direction(*current, value);
        if direction == *current {
            return None;
        }
        *current = direction;
        if direction == 0 {
            self.held.remove(&source);
            return None;
        }
        self.press(source, if direction < 0 { negative } else { positive }, now)
    }

    fn due_repeats(&mut self, now: Instant) -> Vec<NavigationAction> {
        let mut actions = Vec::new();
        for held in self.held.values_mut() {
            if now < held.next_repeat {
                continue;
            }
            if !actions.contains(&held.action) {
                actions.push(held.action);
            }
            held.next_repeat = now + REPEAT_INTERVAL;
        }
        actions
    }

    fn clear(&mut self) {
        self.held.clear();
        self.horizontal_axis = 0;
        self.vertical_axis = 0;
    }
}

#[derive(Debug)]
struct ControllerSnapshot {
    connected_count: i32,
    active_id: Option<GamepadId>,
    active_device: String,
    layout: String,
    status: String,
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::GamepadInput {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() {
            return;
        }
        self.as_mut().set_initialized(true);

        if gamepad_ui_probe_enabled() {
            self.as_mut().set_ready(true);
            self.as_mut().set_available(true);
            self.as_mut().set_connected_count(1);
            self.as_mut()
                .set_active_device(qstring("Couch Mode Probe Controller"));
            self.as_mut().set_controller_layout(qstring("xbox"));
            self.as_mut().set_status_message(qstring(
                "Controller routing probe ready · deterministic Xbox layout",
            ));
            return;
        }

        self.as_mut()
            .set_status_message(qstring("Starting cross-platform gamepad input…"));
        let stop = Arc::clone(&self.as_ref().rust().stop);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-gamepad-input".into())
            .spawn(move || {
                let mut gilrs = match GilrsBuilder::new().with_force_feedback(false).build() {
                    Ok(gilrs) => gilrs,
                    Err(error) => {
                        let message = format!("Gamepad input is unavailable: {error}");
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().set_ready(true);
                            model.as_mut().set_available(false);
                            model.as_mut().set_status_message(qstring(message));
                        });
                        return;
                    }
                };

                let mut active_id = None;
                let snapshot = controller_snapshot(&gilrs, active_id);
                active_id = snapshot.active_id;
                if qt_thread
                    .queue(move |mut model| {
                        model.as_mut().apply_snapshot(snapshot);
                        model.as_mut().set_ready(true);
                        model.as_mut().set_available(true);
                    })
                    .is_err()
                {
                    return;
                }

                let mut translator = NavigationTranslator::default();
                while !stop.load(Ordering::Acquire) {
                    let event = gilrs.next_event_blocking(Some(EVENT_POLL_INTERVAL));
                    let now = Instant::now();
                    if let Some(event) = event {
                        let connection_changed =
                            matches!(event.event, EventType::Connected | EventType::Disconnected);
                        if matches!(event.event, EventType::Connected) {
                            active_id = Some(event.id);
                        } else if matches!(event.event, EventType::Disconnected)
                            && active_id == Some(event.id)
                        {
                            active_id = None;
                        }

                        if connection_changed {
                            let snapshot = controller_snapshot(&gilrs, active_id);
                            active_id = snapshot.active_id;
                            if qt_thread
                                .queue(move |mut model| {
                                    model.as_mut().apply_snapshot(snapshot);
                                })
                                .is_err()
                            {
                                break;
                            }
                        }

                        if let Some(action) = translator.handle_event(event.event, now) {
                            active_id = Some(event.id);
                            let gamepad = gilrs.gamepad(event.id);
                            let device = gamepad.name().to_owned();
                            let layout = infer_controller_layout(&device, gamepad.vendor_id());
                            if qt_thread
                                .queue(move |mut model| {
                                    model.as_mut().publish_action(action, device, layout);
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    }

                    for action in translator.due_repeats(now) {
                        let (device, layout) = active_id
                            .map(|id| {
                                let gamepad = gilrs.gamepad(id);
                                let device = gamepad.name().to_owned();
                                let layout = infer_controller_layout(&device, gamepad.vendor_id());
                                (device, layout)
                            })
                            .unwrap_or_else(|| (String::new(), "generic".to_owned()));
                        if qt_thread
                            .queue(move |mut model| {
                                model.as_mut().publish_action(action, device, layout);
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    gilrs.inc();
                }
            });

        if let Err(error) = spawn {
            self.as_mut().set_ready(true);
            self.as_mut().set_available(false);
            self.as_mut()
                .set_status_message(qstring(format!("Could not start gamepad input: {error}")));
        }
    }

    pub fn button_label(&self, action: QString) -> QString {
        qstring(controller_button_label(
            self.controller_layout().to_string().as_str(),
            action.to_string().as_str(),
        ))
    }

    pub fn probe_navigation_action(mut self: Pin<&mut Self>, action: QString) {
        if !gamepad_ui_probe_enabled() {
            return;
        }
        let Some(action) = NavigationAction::parse(action.to_string().as_str()) else {
            return;
        };
        self.as_mut().publish_action(
            action,
            "Couch Mode Probe Controller".to_owned(),
            "xbox".to_owned(),
        );
    }

    fn apply_snapshot(mut self: Pin<&mut Self>, snapshot: ControllerSnapshot) {
        self.as_mut().set_connected_count(snapshot.connected_count);
        self.as_mut()
            .set_active_device(qstring(snapshot.active_device));
        self.as_mut()
            .set_controller_layout(qstring(snapshot.layout));
        self.as_mut().set_status_message(qstring(snapshot.status));
    }

    fn publish_action(
        mut self: Pin<&mut Self>,
        action: NavigationAction,
        device: String,
        layout: String,
    ) {
        if !device.is_empty() && self.as_ref().active_device().to_string() != device {
            self.as_mut().set_active_device(qstring(device));
        }
        if self.as_ref().controller_layout().to_string() != layout {
            self.as_mut().set_controller_layout(qstring(layout));
        }
        self.as_mut()
            .set_navigation_action(qstring(action.as_str()));
        let revision = self.as_ref().navigation_revision().wrapping_add(1);
        self.as_mut().set_navigation_revision(revision);
    }
}

fn controller_snapshot(gilrs: &Gilrs, preferred: Option<GamepadId>) -> ControllerSnapshot {
    let connected = gilrs
        .gamepads()
        .map(|(id, gamepad)| (id, gamepad.name().to_owned(), gamepad.vendor_id()))
        .collect::<Vec<_>>();
    let active = preferred
        .and_then(|preferred| {
            connected
                .iter()
                .find(|(id, _, _)| *id == preferred)
                .cloned()
        })
        .or_else(|| connected.first().cloned());
    let (active_id, active_device, layout) = active
        .map(|(id, name, vendor)| {
            let layout = infer_controller_layout(&name, vendor);
            (Some(id), name, layout)
        })
        .unwrap_or_else(|| (None, String::new(), "generic".to_owned()));
    let connected_count = i32::try_from(connected.len()).unwrap_or(i32::MAX);
    let status = match connected_count {
        0 => "Listening for a controller.".to_owned(),
        1 => format!("1 controller connected · {active_device}"),
        count => format!("{count} controllers connected · {active_device} active"),
    };
    ControllerSnapshot {
        connected_count,
        active_id,
        active_device,
        layout,
        status,
    }
}

fn button_action(button: Button) -> Option<NavigationAction> {
    match button {
        Button::DPadUp => Some(NavigationAction::Up),
        Button::DPadDown => Some(NavigationAction::Down),
        Button::DPadLeft => Some(NavigationAction::Left),
        Button::DPadRight => Some(NavigationAction::Right),
        Button::LeftTrigger | Button::LeftTrigger2 => Some(NavigationAction::PageLeft),
        Button::RightTrigger | Button::RightTrigger2 => Some(NavigationAction::PageRight),
        Button::South => Some(NavigationAction::Accept),
        Button::East => Some(NavigationAction::Back),
        Button::West => Some(NavigationAction::Favorite),
        Button::North => Some(NavigationAction::Details),
        Button::Start => Some(NavigationAction::Menu),
        Button::Select => Some(NavigationAction::Home),
        _ => None,
    }
}

fn axis_direction(current: i8, value: f32) -> i8 {
    if value >= AXIS_ENGAGE {
        1
    } else if value <= -AXIS_ENGAGE {
        -1
    } else if value.abs() <= AXIS_RELEASE {
        0
    } else {
        current
    }
}

fn infer_controller_layout(name: &str, vendor_id: Option<u16>) -> String {
    let name = name.to_lowercase();
    if vendor_id == Some(0x054c)
        || ["playstation", "dualshock", "dualsense", "sony"]
            .iter()
            .any(|needle| name.contains(needle))
    {
        "playstation".to_owned()
    } else if vendor_id == Some(0x057e)
        || ["nintendo", "switch", "joy-con"]
            .iter()
            .any(|needle| name.contains(needle))
    {
        "nintendo".to_owned()
    } else if vendor_id == Some(0x045e)
        || ["xbox", "x-box", "xinput", "microsoft"]
            .iter()
            .any(|needle| name.contains(needle))
    {
        "xbox".to_owned()
    } else {
        "generic".to_owned()
    }
}

fn controller_button_label(layout: &str, action: &str) -> &'static str {
    match (layout, action) {
        ("playstation", "accept") => "CROSS",
        ("playstation", "back") => "CIRCLE",
        ("playstation", "favorite") => "SQUARE",
        ("playstation", "details") => "TRIANGLE",
        ("playstation", "page_left") => "L1",
        ("playstation", "page_right") => "R1",
        ("playstation", "menu") => "OPTIONS",
        ("playstation", "home") => "CREATE",
        ("nintendo", "accept") => "B",
        ("nintendo", "back") => "A",
        ("nintendo", "favorite") => "Y",
        ("nintendo", "details") => "X",
        ("nintendo", "page_left") => "L",
        ("nintendo", "page_right") => "R",
        ("nintendo", "menu") => "+",
        ("nintendo", "home") => "−",
        ("xbox", "accept") => "A",
        ("xbox", "back") => "B",
        ("xbox", "favorite") => "X",
        ("xbox", "details") => "Y",
        ("xbox", "page_left") => "LB",
        ("xbox", "page_right") => "RB",
        ("xbox", "menu") => "MENU",
        ("xbox", "home") => "VIEW",
        (_, "accept") => "SOUTH",
        (_, "back") => "EAST",
        (_, "favorite") => "WEST",
        (_, "details") => "NORTH",
        (_, "page_left") => "L1",
        (_, "page_right") => "R1",
        (_, "menu") => "START",
        (_, "home") => "SELECT",
        _ => "BUTTON",
    }
}

fn gamepad_ui_probe_enabled() -> bool {
    std::env::args().any(|argument| argument == "--couch-gamepad-ui-probe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_buttons_cover_every_couch_action_without_repeating_commands() {
        assert_eq!(button_action(Button::DPadUp), Some(NavigationAction::Up));
        assert_eq!(button_action(Button::South), Some(NavigationAction::Accept));
        assert_eq!(button_action(Button::East), Some(NavigationAction::Back));
        assert_eq!(
            button_action(Button::West),
            Some(NavigationAction::Favorite)
        );
        assert_eq!(
            button_action(Button::North),
            Some(NavigationAction::Details)
        );
        assert_eq!(
            button_action(Button::LeftTrigger),
            Some(NavigationAction::PageLeft)
        );
        assert_eq!(button_action(Button::LeftThumb), None);

        assert!(!NavigationAction::Accept.repeats());
        assert!(!NavigationAction::Back.repeats());
    }

    #[test]
    fn directional_buttons_repeat_after_a_deliberate_delay_and_stop_on_release() {
        let now = Instant::now();
        let mut translator = NavigationTranslator::default();
        assert_eq!(
            translator.press(
                RepeatSource::Button(Button::DPadRight),
                NavigationAction::Right,
                now,
            ),
            Some(NavigationAction::Right)
        );
        assert!(
            translator
                .due_repeats(now + INITIAL_REPEAT_DELAY - Duration::from_millis(1))
                .is_empty()
        );
        assert_eq!(
            translator.due_repeats(now + INITIAL_REPEAT_DELAY),
            vec![NavigationAction::Right]
        );
        translator
            .held
            .remove(&RepeatSource::Button(Button::DPadRight));
        assert!(
            translator
                .due_repeats(now + INITIAL_REPEAT_DELAY + REPEAT_INTERVAL)
                .is_empty()
        );
    }

    #[test]
    fn analog_navigation_uses_hysteresis_and_supports_hold_repeat() {
        let now = Instant::now();
        let mut translator = NavigationTranslator::default();
        assert_eq!(
            translator.update_axis(Axis::LeftStickX, 0.75, now),
            Some(NavigationAction::Right)
        );
        assert_eq!(translator.update_axis(Axis::LeftStickX, 0.5, now), None);
        assert_eq!(
            translator.due_repeats(now + INITIAL_REPEAT_DELAY),
            vec![NavigationAction::Right]
        );
        assert_eq!(translator.update_axis(Axis::LeftStickX, 0.1, now), None);
        assert!(
            translator
                .due_repeats(now + INITIAL_REPEAT_DELAY + REPEAT_INTERVAL)
                .is_empty()
        );
        assert_eq!(
            translator.update_axis(Axis::LeftStickY, 0.9, now),
            Some(NavigationAction::Up)
        );
    }

    #[test]
    fn physical_button_labels_follow_the_active_controller_family() {
        assert_eq!(
            infer_controller_layout("Wireless Controller", Some(0x054c)),
            "playstation"
        );
        assert_eq!(
            infer_controller_layout("Pro Controller", Some(0x057e)),
            "nintendo"
        );
        assert_eq!(
            infer_controller_layout("Xbox Wireless Controller", None),
            "xbox"
        );
        assert_eq!(controller_button_label("playstation", "accept"), "CROSS");
        assert_eq!(controller_button_label("nintendo", "back"), "A");
        assert_eq!(controller_button_label("xbox", "favorite"), "X");
        assert_eq!(controller_button_label("generic", "details"), "NORTH");
    }
}
