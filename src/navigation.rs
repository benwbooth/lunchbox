use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavigationAction {
    Up,
    Down,
    Left,
    Right,
    Next,
    Previous,
    Activate,
    Back,
}

const NAV_SELECTOR: &str = concat!(
    r#"button:not([disabled]),"#,
    r#"a[href],"#,
    r#"input:not([type="hidden"]):not([disabled]),"#,
    r#"select:not([disabled]),"#,
    r#"textarea:not([disabled]),"#,
    r#"summary,"#,
    r#"[data-nav="true"]:not([disabled]),"#,
    r#"[tabindex]:not([tabindex="-1"]):not([disabled])"#
);

const ACTIVE_SCOPE_SELECTOR: &str = r#"[data-nav-scope][data-nav-scope-active="true"]"#;
const BACK_SELECTOR: &str = r#"[data-nav-back="true"]"#;

pub fn keyboard_action(event: &KeyboardEvent) -> Option<NavigationAction> {
    if event.ctrl_key() || event.meta_key() || event.alt_key() {
        return None;
    }

    match event.key().as_str() {
        "ArrowUp" => Some(NavigationAction::Up),
        "ArrowDown" => Some(NavigationAction::Down),
        "ArrowLeft" => Some(NavigationAction::Left),
        "ArrowRight" => Some(NavigationAction::Right),
        "Tab" if event.shift_key() => Some(NavigationAction::Previous),
        "Tab" => Some(NavigationAction::Next),
        "Enter" | " " | "Spacebar" => Some(NavigationAction::Activate),
        "Escape" => Some(NavigationAction::Back),
        _ => None,
    }
}

pub fn should_ignore_keyboard_action(event: &KeyboardEvent, action: NavigationAction) -> bool {
    let Some(target) = event
        .target()
        .and_then(|target| target.dyn_into::<Element>().ok())
    else {
        return false;
    };

    if !is_text_entry_element(&target) {
        return false;
    }

    matches!(
        action,
        NavigationAction::Up
            | NavigationAction::Down
            | NavigationAction::Left
            | NavigationAction::Right
            | NavigationAction::Activate
    )
}

pub fn handle_navigation_action(action: NavigationAction) -> bool {
    match action {
        NavigationAction::Activate => activate_current(),
        NavigationAction::Back => invoke_back_action(),
        NavigationAction::Next | NavigationAction::Previous => move_linear(action),
        NavigationAction::Up
        | NavigationAction::Down
        | NavigationAction::Left
        | NavigationAction::Right => move_directional(action),
    }
}

fn move_linear(action: NavigationAction) -> bool {
    let Some(document) = document() else {
        return false;
    };
    let active_scope = active_scope_root(&document);
    let candidates = navigation_candidates(&document, active_scope.as_ref());
    if candidates.is_empty() {
        return false;
    }

    let Some(current) = current_navigation_element(&document, active_scope.as_ref()) else {
        return match action {
            NavigationAction::Next => candidates.first().map(focus_candidate).unwrap_or(false),
            NavigationAction::Previous => candidates.last().map(focus_candidate).unwrap_or(false),
            _ => false,
        };
    };

    let current_index = candidates
        .iter()
        .position(|candidate| same_element(candidate, &current))
        .unwrap_or(0);
    let next_index = match action {
        NavigationAction::Next => (current_index + 1) % candidates.len(),
        NavigationAction::Previous => {
            if current_index == 0 {
                candidates.len().saturating_sub(1)
            } else {
                current_index - 1
            }
        }
        _ => current_index,
    };

    focus_candidate(&candidates[next_index])
}

fn move_directional(action: NavigationAction) -> bool {
    let Some(document) = document() else {
        return false;
    };
    let active_scope = active_scope_root(&document);
    let candidates = navigation_candidates(&document, active_scope.as_ref());
    if candidates.is_empty() {
        return false;
    }

    let Some(current) = current_navigation_element(&document, active_scope.as_ref()) else {
        return focus_default_candidate(&candidates, active_scope.as_ref());
    };

    if handle_game_grid_direction(&current, action) {
        return true;
    }

    if let Some(next) = find_directional_candidate(&current, &candidates, action) {
        return focus_candidate(&next);
    }

    if let Some(fallback) = find_nearest_candidate(&current, &candidates) {
        return focus_candidate(&fallback);
    }

    false
}

fn activate_current() -> bool {
    let Some(document) = document() else {
        return false;
    };
    let active_scope = active_scope_root(&document);
    let Some(current) = current_navigation_element(&document, active_scope.as_ref()) else {
        let candidates = navigation_candidates(&document, active_scope.as_ref());
        return focus_default_candidate(&candidates, active_scope.as_ref());
    };

    current.click();
    true
}

fn invoke_back_action() -> bool {
    let Some(document) = document() else {
        return false;
    };

    let active_scope = active_scope_root(&document);
    let back_target = if let Some(scope) = active_scope.as_ref() {
        query_selector(scope, BACK_SELECTOR)
    } else {
        query_selector(&document, BACK_SELECTOR)
    };

    if let Some(target) = back_target.and_then(html_element_from) {
        target.click();
        return true;
    }

    false
}

fn handle_game_grid_direction(current: &HtmlElement, action: NavigationAction) -> bool {
    if current.get_attribute("data-nav-kind").as_deref() != Some("game-item") {
        return false;
    }

    let Some(container) = current.closest(r#"[data-nav-grid="true"]"#).ok().flatten() else {
        return false;
    };

    let Some(current_index) = parse_usize_attr(current, "data-game-index") else {
        return false;
    };
    let loaded_count = parse_usize_attr(&container, "data-nav-game-count").unwrap_or(0);
    if loaded_count == 0 {
        return false;
    }

    let view_mode = container
        .get_attribute("data-nav-view-mode")
        .unwrap_or_else(|| "grid".to_string());
    let cols = parse_usize_attr(&container, "data-nav-grid-cols")
        .unwrap_or(1)
        .max(1);
    let row_height = parse_i32_attr(&container, "data-nav-grid-row-height").unwrap_or(280);
    let list_row_height = parse_i32_attr(&container, "data-nav-list-row-height").unwrap_or(40);

    let next_index = if view_mode == "list" {
        match action {
            NavigationAction::Up => current_index.checked_sub(1),
            NavigationAction::Down if current_index + 1 < loaded_count => Some(current_index + 1),
            _ => None,
        }
    } else {
        match action {
            NavigationAction::Up if current_index >= cols => Some(current_index - cols),
            NavigationAction::Down => next_grid_down_index(current_index, cols, loaded_count),
            NavigationAction::Left if current_index % cols != 0 => Some(current_index - 1),
            NavigationAction::Right if current_index + 1 < loaded_count => Some(current_index + 1),
            _ => None,
        }
    };

    let Some(next_index) = next_index else {
        return false;
    };
    if next_index == current_index {
        return false;
    }

    focus_game_grid_index(
        container,
        next_index,
        &view_mode,
        cols,
        row_height,
        list_row_height,
    );
    true
}

fn next_grid_down_index(current_index: usize, cols: usize, loaded_count: usize) -> Option<usize> {
    if loaded_count == 0 {
        return None;
    }

    let direct = current_index + cols;
    if direct < loaded_count {
        return Some(direct);
    }

    let last_row_start = ((loaded_count - 1) / cols) * cols;
    if current_index >= last_row_start {
        return None;
    }

    let column = current_index % cols;
    let candidate = last_row_start + column;
    if candidate < loaded_count {
        Some(candidate)
    } else {
        Some(loaded_count - 1)
    }
}

fn focus_game_grid_index(
    container: Element,
    next_index: usize,
    view_mode: &str,
    cols: usize,
    row_height: i32,
    list_row_height: i32,
) {
    if let Some(container_html) = html_element_from(container.clone()) {
        let scroll_top = if view_mode == "list" {
            (next_index as i32 + 1) * list_row_height
        } else {
            (next_index as i32 / cols as i32) * row_height
        };
        container_html.set_scroll_top(scroll_top.max(0));
    }

    let selector = format!(
        r#"[data-nav-kind="game-item"][data-game-index="{}"]"#,
        next_index
    );
    if let Some(element) = query_selector(&container, &selector).and_then(html_element_from) {
        let _ = focus_candidate(&element);
        return;
    }

    spawn_local(async move {
        for _ in 0..12 {
            delay_ms(16).await;
            if let Some(element) = query_selector(&container, &selector).and_then(html_element_from)
            {
                let _ = focus_candidate(&element);
                return;
            }
        }
    });
}

async fn delay_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        } else {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

fn find_directional_candidate(
    current: &HtmlElement,
    candidates: &[HtmlElement],
    action: NavigationAction,
) -> Option<HtmlElement> {
    let current_rect = current.get_bounding_client_rect();

    candidates
        .iter()
        .filter(|candidate| !same_element(candidate, current))
        .filter_map(|candidate| {
            let rect = candidate.get_bounding_client_rect();
            let (primary_gap, secondary_gap, valid) =
                directional_gaps(&current_rect, &rect, action);
            valid.then_some((primary_gap, secondary_gap, candidate.clone()))
        })
        .min_by(|(primary_a, secondary_a, _), (primary_b, secondary_b, _)| {
            primary_a
                .partial_cmp(primary_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    secondary_a
                        .partial_cmp(secondary_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .map(|(_, _, candidate)| candidate)
}

fn find_nearest_candidate(
    current: &HtmlElement,
    candidates: &[HtmlElement],
) -> Option<HtmlElement> {
    let current_rect = current.get_bounding_client_rect();
    let current_center_x = current_rect.left() + current_rect.width() / 2.0;
    let current_center_y = current_rect.top() + current_rect.height() / 2.0;

    candidates
        .iter()
        .filter(|candidate| !same_element(candidate, current))
        .min_by(|a, b| {
            let rect_a = a.get_bounding_client_rect();
            let rect_b = b.get_bounding_client_rect();
            let distance_a = squared_distance(
                current_center_x,
                current_center_y,
                rect_a.left() + rect_a.width() / 2.0,
                rect_a.top() + rect_a.height() / 2.0,
            );
            let distance_b = squared_distance(
                current_center_x,
                current_center_y,
                rect_b.left() + rect_b.width() / 2.0,
                rect_b.top() + rect_b.height() / 2.0,
            );
            distance_a
                .partial_cmp(&distance_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

fn directional_gaps(
    current: &web_sys::DomRect,
    candidate: &web_sys::DomRect,
    action: NavigationAction,
) -> (f64, f64, bool) {
    let horizontal_gap = if candidate.right() < current.left() {
        current.left() - candidate.right()
    } else if candidate.left() > current.right() {
        candidate.left() - current.right()
    } else {
        0.0
    };
    let vertical_gap = if candidate.bottom() < current.top() {
        current.top() - candidate.bottom()
    } else if candidate.top() > current.bottom() {
        candidate.top() - current.bottom()
    } else {
        0.0
    };

    match action {
        NavigationAction::Up => (
            if candidate.bottom() <= current.top() {
                current.top() - candidate.bottom()
            } else {
                0.0
            },
            horizontal_gap,
            candidate.bottom() <= current.top(),
        ),
        NavigationAction::Down => (
            if candidate.top() >= current.bottom() {
                candidate.top() - current.bottom()
            } else {
                0.0
            },
            horizontal_gap,
            candidate.top() >= current.bottom(),
        ),
        NavigationAction::Left => (
            if candidate.right() <= current.left() {
                current.left() - candidate.right()
            } else {
                0.0
            },
            vertical_gap,
            candidate.right() <= current.left(),
        ),
        NavigationAction::Right => (
            if candidate.left() >= current.right() {
                candidate.left() - current.right()
            } else {
                0.0
            },
            vertical_gap,
            candidate.left() >= current.right(),
        ),
        _ => (0.0, 0.0, false),
    }
}

fn squared_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

fn focus_default_candidate(candidates: &[HtmlElement], scope: Option<&Element>) -> bool {
    if let Some(scope) = scope {
        if let Some(default_candidate) =
            query_selector(scope, r#"[data-nav-default="true"]"#).and_then(html_element_from)
        {
            return focus_candidate(&default_candidate);
        }
    } else if let Some(game_item) = candidates
        .iter()
        .find(|candidate| candidate.get_attribute("data-nav-kind").as_deref() == Some("game-item"))
    {
        return focus_candidate(game_item);
    }

    candidates.first().map(focus_candidate).unwrap_or(false)
}

fn focus_candidate(candidate: &HtmlElement) -> bool {
    candidate.scroll_into_view();
    candidate.focus().is_ok()
}

fn current_navigation_element(document: &Document, scope: Option<&Element>) -> Option<HtmlElement> {
    let active = html_element_from(document.active_element()?)?;
    if !is_navigation_candidate(&active) {
        return None;
    }

    if let Some(scope) = scope {
        if !is_descendant_of(&active, scope) {
            return None;
        }
    }

    Some(active)
}

fn active_scope_root(document: &Document) -> Option<Element> {
    query_selector_all(document, ACTIVE_SCOPE_SELECTOR)
        .into_iter()
        .filter(|scope| {
            html_element_from(scope.clone()).is_some_and(|el| is_navigation_candidate(&el))
        })
        .max_by(|a, b| {
            scope_priority(a).cmp(&scope_priority(b)).then_with(|| {
                a.get_attribute("data-nav-scope")
                    .cmp(&b.get_attribute("data-nav-scope"))
            })
        })
}

fn scope_priority(scope: &Element) -> i32 {
    scope
        .get_attribute("data-nav-scope-priority")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
}

fn navigation_candidates(document: &Document, scope: Option<&Element>) -> Vec<HtmlElement> {
    let elements = if let Some(scope) = scope {
        query_selector_all(scope, NAV_SELECTOR)
    } else {
        query_selector_all(document, NAV_SELECTOR)
    };

    elements
        .into_iter()
        .filter_map(html_element_from)
        .filter(is_navigation_candidate)
        .collect()
}

fn query_selector(root: &impl QuerySelectorExt, selector: &str) -> Option<Element> {
    root.query_selector(selector).ok().flatten()
}

fn query_selector_all(root: &impl QuerySelectorExt, selector: &str) -> Vec<Element> {
    let Ok(list) = root.query_selector_all(selector) else {
        return Vec::new();
    };

    let mut elements = Vec::new();
    for index in 0..list.length() {
        if let Some(node) = list.item(index) {
            if let Ok(element) = node.dyn_into::<Element>() {
                elements.push(element);
            }
        }
    }
    elements
}

trait QuerySelectorExt {
    fn query_selector(&self, selector: &str) -> Result<Option<Element>, wasm_bindgen::JsValue>;
    fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<web_sys::NodeList, wasm_bindgen::JsValue>;
}

impl QuerySelectorExt for Document {
    fn query_selector(&self, selector: &str) -> Result<Option<Element>, wasm_bindgen::JsValue> {
        Document::query_selector(self, selector)
    }

    fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<web_sys::NodeList, wasm_bindgen::JsValue> {
        Document::query_selector_all(self, selector)
    }
}

impl QuerySelectorExt for Element {
    fn query_selector(&self, selector: &str) -> Result<Option<Element>, wasm_bindgen::JsValue> {
        Element::query_selector(self, selector)
    }

    fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<web_sys::NodeList, wasm_bindgen::JsValue> {
        Element::query_selector_all(self, selector)
    }
}

fn document() -> Option<Document> {
    web_sys::window()?.document()
}

fn html_element_from(element: Element) -> Option<HtmlElement> {
    element.dyn_into::<HtmlElement>().ok()
}

fn parse_usize_attr(element: &impl AttributeExt, name: &str) -> Option<usize> {
    element.get_attribute(name)?.parse::<usize>().ok()
}

fn parse_i32_attr(element: &impl AttributeExt, name: &str) -> Option<i32> {
    element.get_attribute(name)?.parse::<i32>().ok()
}

trait AttributeExt {
    fn get_attribute(&self, name: &str) -> Option<String>;
}

impl AttributeExt for Element {
    fn get_attribute(&self, name: &str) -> Option<String> {
        Element::get_attribute(self, name)
    }
}

impl AttributeExt for HtmlElement {
    fn get_attribute(&self, name: &str) -> Option<String> {
        Element::get_attribute(self, name)
    }
}

fn is_navigation_candidate(element: &HtmlElement) -> bool {
    if element.get_attribute("aria-hidden").as_deref() == Some("true") {
        return false;
    }
    if element.has_attribute("hidden") {
        return false;
    }

    let rect = element.get_bounding_client_rect();
    rect.width() > 0.0 && rect.height() > 0.0
}

fn is_text_entry_element(element: &Element) -> bool {
    let tag = element.tag_name().to_ascii_lowercase();
    match tag.as_str() {
        "textarea" => true,
        "input" => {
            let input_type = element
                .get_attribute("type")
                .unwrap_or_else(|| "text".to_string())
                .to_ascii_lowercase();
            !matches!(
                input_type.as_str(),
                "button"
                    | "checkbox"
                    | "color"
                    | "file"
                    | "hidden"
                    | "image"
                    | "radio"
                    | "range"
                    | "reset"
                    | "submit"
            )
        }
        _ => element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false"),
    }
}

fn is_descendant_of(element: &HtmlElement, ancestor: &Element) -> bool {
    element
        .closest(&format!(
            r#"[data-nav-scope="{}"]"#,
            ancestor.get_attribute("data-nav-scope").unwrap_or_default()
        ))
        .ok()
        .flatten()
        .is_some_and(|scope| same_element(&scope, ancestor))
}

fn same_element(
    a: &impl AsRef<wasm_bindgen::JsValue>,
    b: &impl AsRef<wasm_bindgen::JsValue>,
) -> bool {
    a.as_ref() == b.as_ref()
}
