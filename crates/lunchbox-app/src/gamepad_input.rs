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
        #[qproperty(QString, last_input)]
        #[qproperty(QString, last_control)]
        #[qproperty(QString, last_device_key)]
        #[qproperty(QString, last_binding)]
        #[qproperty(QString, neutral_device_key)]
        #[qproperty(QString, neutral_binding)]
        #[qproperty(QString, neutral_error)]
        #[qproperty(i32, neutral_revision)]
        #[qproperty(i32, input_revision)]
        #[qproperty(bool, navigation_enabled)]
        #[qproperty(QString, navigation_action)]
        #[qproperty(i32, navigation_revision)]
        type GamepadInput = super::GamepadInputRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut GamepadInput>);
        #[qinvokable]
        fn sync_navigation_enabled(self: &GamepadInput);

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
    last_input: QString,
    last_control: QString,
    last_device_key: QString,
    last_binding: QString,
    neutral_device_key: QString,
    neutral_binding: QString,
    neutral_error: QString,
    neutral_revision: i32,
    input_revision: i32,
    navigation_enabled: bool,
    navigation_action: QString,
    navigation_revision: i32,
    stop: Arc<AtomicBool>,
    navigation_gate: Arc<AtomicBool>,
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
            status_message: QString::from("Controller input initializes in the background."),
            last_input: QString::default(),
            last_control: QString::default(),
            last_device_key: QString::default(),
            last_binding: QString::default(),
            neutral_device_key: QString::default(),
            neutral_binding: QString::default(),
            neutral_error: QString::default(),
            neutral_revision: 0,
            input_revision: 0,
            navigation_enabled: true,
            navigation_action: QString::default(),
            navigation_revision: 0,
            stop: Arc::new(AtomicBool::new(false)),
            navigation_gate: Arc::new(AtomicBool::new(true)),
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
    ScrollFirst,
    ScrollLast,
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
            Self::ScrollFirst => "scroll_first",
            Self::ScrollLast => "scroll_last",
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
            "scroll_first" => Some(Self::ScrollFirst),
            "scroll_last" => Some(Self::ScrollLast),
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
    pub fn sync_navigation_enabled(&self) {
        self.rust()
            .navigation_gate
            .store(*self.navigation_enabled(), Ordering::Release);
    }
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
        let navigation_gate = Arc::clone(&self.as_ref().rust().navigation_gate);
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

                let mut translators: HashMap<GamepadId, NavigationTranslator> = HashMap::new();
                let mut diagnostics: HashMap<GamepadId, DiagnosticTracker> = HashMap::new();
                let mut captures: HashMap<GamepadId, PendingCalibration> = HashMap::new();
                while !stop.load(Ordering::Acquire) {
                    let event = gilrs.next_event_blocking(Some(EVENT_POLL_INTERVAL));
                    let now = Instant::now();
                    let navigation_enabled = navigation_gate.load(Ordering::Acquire);
                    if !navigation_enabled {
                        translators.clear();
                    }
                    if let Some(event) = event {
                        let (binding, neutral) =
                            diagnostics.entry(event.id).or_default().handle(event.event);
                        if neutral && let Some(capture) = captures.get_mut(&event.id) {
                            capture.release(now);
                        }
                        if let Some(mut binding) = binding {
                            binding.native = native_input(&gilrs.gamepad(event.id), event.event);
                            let key = input_device_key(&gilrs.gamepad(event.id));
                            captures.entry(event.id).or_insert_with(|| PendingCalibration::new(binding.clone(), &key));
                            let control = binding.logical.clone();
                            let encoded =
                                serde_json::to_string(&binding).expect("input binding serializes");
                            let name = gilrs.gamepad(event.id).name().to_owned();
                            let number = usize::from(event.id) + 1;
                            let description = format!("Pad {number}: {name} · {control}");
                            let device_key = input_device_key(&gilrs.gamepad(event.id));
                            let _ = qt_thread.queue(move |mut model| {
                                model.as_mut().set_last_device_key(qstring(device_key));
                                model.as_mut().set_last_binding(qstring(encoded));
                                model.as_mut().set_last_input(qstring(description));
                                model.as_mut().set_last_control(qstring(control));
                                let revision = model.as_ref().input_revision().wrapping_add(1);
                                model.as_mut().set_input_revision(revision);
                            });
                        }
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
                            diagnostics.remove(&event.id);
                            translators.remove(&event.id);
                            if let Some(mut capture) = captures.remove(&event.id) {
                                capture.error = Some("Controller disconnected. Reconnect it and repeat this control.".into());
                                let key = input_device_key(&gilrs.gamepad(event.id));
                                let error = capture.error.unwrap();
                                let _ = qt_thread.queue(move |mut model| {
                                    model.as_mut().set_neutral_device_key(qstring(key));
                                    model.as_mut().set_neutral_binding(qstring(""));
                                    model.as_mut().set_neutral_error(qstring(error));
                                    let revision = model.as_ref().neutral_revision().wrapping_add(1);
                                    model.as_mut().set_neutral_revision(revision);
                                });
                            }
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

                        if navigation_enabled
                            && let Some(action) = translators
                                .entry(event.id)
                                .or_default()
                                .handle_event(event.event, now)
                        {
                            if active_id != Some(event.id) {
                                for (id, translator) in &mut translators {
                                    if *id != event.id {
                                        translator.clear();
                                    }
                                }
                            }
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

                    captures.retain(|id, capture| {
                        let Some(completed) = capture.poll(now) else { return true; };
                        let key = input_device_key(&gilrs.gamepad(*id));
                        let (binding, error) = match completed {
                            Ok(binding) => (serde_json::to_string(&binding).expect("binding serializes"), String::new()),
                            Err(error) => (String::new(), error),
                        };
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().set_neutral_device_key(qstring(key));
                            model.as_mut().set_neutral_binding(qstring(binding));
                            model.as_mut().set_neutral_error(qstring(error));
                            let revision = model.as_ref().neutral_revision().wrapping_add(1);
                            model.as_mut().set_neutral_revision(revision);
                        });
                        false
                    });

                    let repeats = active_id
                        .and_then(|id| translators.get_mut(&id))
                        .map(|translator| translator.due_repeats(now))
                        .unwrap_or_default();
                    for action in repeats {
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
        if !*self.as_ref().navigation_enabled() {
            return;
        }
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

struct PendingCalibration {
    binding: crate::controller_catalog::InputBinding,
    axis: Option<crate::controller_axis::AxisCapture>,
    released: bool,
    error: Option<String>,
}

impl PendingCalibration {
    fn new(binding: crate::controller_catalog::InputBinding, device: &str) -> Self {
        let mut result = Self {
            binding,
            axis: None,
            released: false,
            error: None,
        };
        if let Some(native) = &result.binding.native
            && native.code >> 16 == 3
        {
            match crate::controller_axis::AxisCapture::open(
                std::path::Path::new(device),
                native.code,
            ) {
                Ok(axis) => result.axis = Some(axis),
                Err(error) => {
                    result.error = Some(format!(
                        "Could not measure this physical axis: {error}. Try again."
                    ))
                }
            }
        }
        result
    }

    fn release(&mut self, now: Instant) {
        self.released = true;
        if let Some(axis) = &mut self.axis {
            axis.release(now);
        }
    }

    fn poll(
        &mut self,
        now: Instant,
    ) -> Option<Result<crate::controller_catalog::InputBinding, String>> {
        if self.error.is_none()
            && let Some(axis) = &mut self.axis
        {
            match axis.poll(now) {
                Ok(Some(measured)) => {
                    if let Some(native) = &mut self.binding.native {
                        // Raw measured motion, not a guess based on a normalized
                        // Xbox/GilRs label (which may be inverted by its mapping).
                        native.direction = measured.direction();
                    }
                    self.binding.axis = Some(measured);
                    self.axis = None;
                }
                Ok(None) => return None,
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if !self.released {
            return None;
        }
        Some(match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(self.binding.clone()),
        })
    }
}

#[derive(Default)]
struct DiagnosticTracker {
    buttons: std::collections::HashSet<u32>,
    axes: HashMap<u32, i8>,
}

impl DiagnosticTracker {
    fn handle(
        &mut self,
        event: EventType,
    ) -> (Option<crate::controller_catalog::InputBinding>, bool) {
        let was_active = !self.buttons.is_empty() || !self.axes.is_empty();
        let native = match event {
            EventType::ButtonPressed(_, code) if self.buttons.insert(code.into_u32()) => {
                Some((code.into_u32(), "button", 0))
            }
            EventType::ButtonReleased(_, code) => {
                self.buttons.remove(&code.into_u32());
                None
            }
            EventType::AxisChanged(axis, value, code)
                if matches!(
                    axis,
                    Axis::LeftStickX | Axis::LeftStickY | Axis::RightStickX | Axis::RightStickY
                ) =>
            {
                let code = code.into_u32();
                if value.abs() <= AXIS_RELEASE {
                    self.axes.remove(&code);
                    None
                } else if value.abs() >= AXIS_ENGAGE {
                    let direction = if value < 0.0 { -1 } else { 1 };
                    if self.axes.insert(code, direction) != Some(direction) {
                        Some((code, "axis", direction))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            EventType::Disconnected => {
                self.buttons.clear();
                self.axes.clear();
                None
            }
            _ => None,
        };
        let binding = native.and_then(|(code, kind, direction)| {
            diagnostic_control(event).map(|logical| crate::controller_catalog::InputBinding {
                code,
                kind: kind.into(),
                direction,
                logical,
                native: None,
                axis: None,
            })
        });
        (
            binding,
            was_active && self.buttons.is_empty() && self.axes.is_empty(),
        )
    }
}

fn diagnostic_control(event: EventType) -> Option<String> {
    match event {
        EventType::ButtonPressed(Button::Unknown, code) => Some(format!("Unmapped button {code}")),
        EventType::ButtonPressed(button, _) => Some(match button {
            Button::LeftThumb => "LeftStick".into(),
            Button::RightThumb => "RightStick".into(),
            Button::Mode => "Guide".into(),
            Button::LeftTrigger => "LeftBumper".into(),
            Button::RightTrigger => "RightBumper".into(),
            Button::LeftTrigger2 => "LeftTrigger".into(),
            Button::RightTrigger2 => "RightTrigger".into(),
            _ => format!("{button:?}"),
        }),
        EventType::AxisChanged(axis, value, _) if value.abs() >= AXIS_ENGAGE => match axis {
            Axis::LeftStickX => Some(
                if value < 0.0 {
                    "LeftStickLeft"
                } else {
                    "LeftStickRight"
                }
                .into(),
            ),
            Axis::LeftStickY => Some(
                if value < 0.0 {
                    "LeftStickDown"
                } else {
                    "LeftStickUp"
                }
                .into(),
            ),
            Axis::RightStickX => Some(
                if value < 0.0 {
                    "RightStickLeft"
                } else {
                    "RightStickRight"
                }
                .into(),
            ),
            Axis::RightStickY => Some(
                if value < 0.0 {
                    "RightStickDown"
                } else {
                    "RightStickUp"
                }
                .into(),
            ),
            _ => None,
        },
        _ => None,
    }
}

fn input_device_key(gamepad: &gilrs::Gamepad<'_>) -> String {
    #[cfg(target_os = "linux")]
    {
        use gilrs::LinuxGamepadExt;
        gamepad.devpath().to_string_lossy().into_owned()
    }
    #[cfg(not(target_os = "linux"))]
    {
        crate::controllers::portable_input_device_key(&hex::encode(gamepad.uuid()), gamepad.name())
    }
}

fn native_input(
    gamepad: &gilrs::Gamepad<'_>,
    event: EventType,
) -> Option<crate::controller_catalog::NativeInput> {
    #[cfg(target_os = "linux")]
    {
        use crate::controller_catalog::NativeInput;
        match event {
            EventType::ButtonPressed(button, code) => {
                use gilrs::LinuxGamepadExt;
                let packed = code.into_u32();
                let physical_button = packed >> 16 == 1
                    && crate::controller_launch::physical_button_present(
                        gamepad.devpath(),
                        packed as u16,
                    )
                    .ok()?;
                // GilRs' D-pad filter emits synthetic BTN_DPAD codes for hats.
                // Recover the actual mapped hat axis instead of guessing its index.
                let hat = match button {
                    Button::DPadUp => Some((Axis::DPadY, -1)),
                    Button::DPadDown => Some((Axis::DPadY, 1)),
                    Button::DPadLeft => Some((Axis::DPadX, -1)),
                    Button::DPadRight => Some((Axis::DPadX, 1)),
                    _ => None,
                };
                if !physical_button
                    && let Some((axis, direction)) = hat
                    && let Some(axis_code) = gamepad.axis_code(axis)
                {
                    return Some(NativeInput {
                        code: axis_code.into_u32(),
                        direction,
                    });
                }
                let code = code.into_u32();
                Some(NativeInput {
                    code,
                    direction: if code >> 16 == 3 { 1 } else { 0 },
                })
            }
            EventType::AxisChanged(axis, value, code) => {
                let sign = if value < 0.0 { -1 } else { 1 };
                Some(NativeInput {
                    code: code.into_u32(),
                    direction: if matches!(axis, Axis::LeftStickY | Axis::RightStickY | Axis::DPadY)
                    {
                        -sign
                    } else {
                        sign
                    },
                })
            }
            _ => None,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (gamepad, event);
        None
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
        Button::LeftTrigger => Some(NavigationAction::PageLeft),
        Button::RightTrigger => Some(NavigationAction::PageRight),
        Button::LeftTrigger2 => Some(NavigationAction::ScrollFirst),
        Button::RightTrigger2 => Some(NavigationAction::ScrollLast),
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
        ("playstation", "scroll_first") => "L2",
        ("playstation", "scroll_last") => "R2",
        ("playstation", "menu") => "OPTIONS",
        ("playstation", "home") => "CREATE",
        ("nintendo", "accept") => "B",
        ("nintendo", "back") => "A",
        ("nintendo", "favorite") => "Y",
        ("nintendo", "details") => "X",
        ("nintendo", "page_left") => "L",
        ("nintendo", "page_right") => "R",
        ("nintendo", "scroll_first") => "ZL",
        ("nintendo", "scroll_last") => "ZR",
        ("nintendo", "menu") => "+",
        ("nintendo", "home") => "−",
        ("xbox", "accept") => "A",
        ("xbox", "back") => "B",
        ("xbox", "favorite") => "X",
        ("xbox", "details") => "Y",
        ("xbox", "page_left") => "LB",
        ("xbox", "page_right") => "RB",
        ("xbox", "scroll_first") => "LT",
        ("xbox", "scroll_last") => "RT",
        ("xbox", "menu") => "MENU",
        ("xbox", "home") => "VIEW",
        (_, "accept") => "SOUTH",
        (_, "back") => "EAST",
        (_, "favorite") => "WEST",
        (_, "details") => "NORTH",
        (_, "page_left") => "L1",
        (_, "page_right") => "R1",
        (_, "scroll_first") => "L2",
        (_, "scroll_last") => "R2",
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
    fn calibration_tracks_button_release_without_repeating_held_inputs() {
        let mut tracker = DiagnosticTracker::default();
        let code = Button::South.to_nec().unwrap();
        let (binding, neutral) = tracker.handle(EventType::ButtonPressed(Button::South, code));
        assert_eq!(binding.unwrap().logical, "South");
        assert!(!neutral);
        assert!(
            tracker
                .handle(EventType::ButtonPressed(Button::South, code))
                .0
                .is_none()
        );
        assert!(
            tracker
                .handle(EventType::ButtonReleased(Button::South, code))
                .1
        );
        assert!(
            !tracker
                .handle(EventType::ButtonReleased(Button::South, code))
                .1
        );
    }

    #[test]
    fn calibration_axis_hysteresis_waits_for_neutral() {
        let mut tracker = DiagnosticTracker::default();
        // The tracker treats native codes as opaque identifiers. This fixture
        // exercises axis events without requiring a physical device in CI.
        let code = Button::South.to_nec().unwrap();
        let (binding, _) = tracker.handle(EventType::AxisChanged(Axis::RightStickY, 0.9, code));
        assert_eq!(binding.unwrap().logical, "RightStickUp");
        assert!(
            tracker
                .handle(EventType::AxisChanged(Axis::RightStickY, 0.8, code))
                .0
                .is_none()
        );
        assert!(
            !tracker
                .handle(EventType::AxisChanged(Axis::RightStickY, 0.5, code))
                .1
        );
        assert!(
            tracker
                .handle(EventType::AxisChanged(Axis::RightStickY, 0.1, code))
                .1
        );
        let (binding, _) = tracker.handle(EventType::AxisChanged(Axis::RightStickY, -0.9, code));
        assert_eq!(binding.unwrap().direction, -1);
    }

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
        assert_eq!(
            button_action(Button::RightTrigger),
            Some(NavigationAction::PageRight)
        );
        assert_eq!(
            button_action(Button::LeftTrigger2),
            Some(NavigationAction::ScrollFirst)
        );
        assert_eq!(
            button_action(Button::RightTrigger2),
            Some(NavigationAction::ScrollLast)
        );
        assert_eq!(
            NavigationAction::parse("scroll_first"),
            Some(NavigationAction::ScrollFirst)
        );
        assert_eq!(
            NavigationAction::parse("scroll_last"),
            Some(NavigationAction::ScrollLast)
        );
        assert!(!NavigationAction::ScrollFirst.repeats());
        assert!(!NavigationAction::ScrollLast.repeats());

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
