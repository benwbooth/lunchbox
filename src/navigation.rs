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

    if current.get_attribute("data-nav-kind").as_deref() == Some("game-grid") {
        return activate_game_grid(&current);
    }

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
    let (container, current_index) = if current.get_attribute("data-nav-kind").as_deref()
        == Some("game-grid")
    {
        let container: Element = current.clone().unchecked_into();
        let loaded_count = parse_usize_attr(&container, "data-nav-game-count").unwrap_or(0);
        if loaded_count == 0 {
            return false;
        }
        let current_index = parse_usize_attr(current, "data-nav-selected-index")
            .unwrap_or(0)
            .min(loaded_count.saturating_sub(1));
        (container, current_index)
    } else {
        if current.get_attribute("data-nav-kind").as_deref() != Some("game-item") {
            return false;
        }
        let Some(container) = current.closest(r#"[data-nav-grid="true"]"#).ok().flatten() else {
            return false;
        };
        let Some(current_index) = parse_usize_attr(current, "data-game-index") else {
            return false;
        };
        (container, current_index)
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
    let maybe_container_html = html_element_from(container.clone());
    let desired_scroll_top = maybe_container_html.as_ref().map(|container_html| {
        reveal_game_grid_index(
            container_html,
            view_mode,
            next_index,
            cols,
            row_height,
            list_row_height,
        )
    });

    if let Some(container_html) = maybe_container_html.as_ref() {
        set_game_grid_selected_index(container_html, next_index);
        focus_game_grid_container(container_html, desired_scroll_top);
        return;
    }
}

fn reveal_game_grid_index(
    container_html: &HtmlElement,
    view_mode: &str,
    next_index: usize,
    cols: usize,
    row_height: i32,
    list_row_height: i32,
) -> i32 {
    let current_scroll_top = container_html.scroll_top().max(0);
    let client_height = container_html.client_height().max(0);
    if client_height <= 0 {
        return current_scroll_top;
    }

    let next_scroll_top = if view_mode == "list" {
        let row_top = list_row_height + next_index as i32 * list_row_height;
        let row_bottom = row_top + list_row_height;
        let viewport_top = current_scroll_top + list_row_height;
        let viewport_bottom = current_scroll_top + client_height;

        if row_top < viewport_top {
            (row_top - list_row_height).max(0)
        } else if row_bottom > viewport_bottom {
            (row_bottom - client_height).max(0)
        } else {
            current_scroll_top
        }
    } else {
        let next_row = next_index / cols.max(1);
        let row_top = next_row as i32 * row_height;
        let row_bottom = row_top + row_height;
        let viewport_top = current_scroll_top;
        let viewport_bottom = current_scroll_top + client_height;

        if row_top < viewport_top {
            row_top.max(0)
        } else if row_bottom > viewport_bottom {
            (row_bottom - client_height).max(0)
        } else {
            current_scroll_top
        }
    };

    if next_scroll_top != current_scroll_top {
        container_html.set_scroll_top(next_scroll_top);
    }

    next_scroll_top
}

fn focus_game_grid_container(container: &HtmlElement, desired_scroll_top: Option<i32>) {
    let _ = focus_without_scroll(container);
    let Some(desired_scroll_top) = desired_scroll_top else {
        return;
    };

    let container = container.clone();
    restore_scroll_top(&container, desired_scroll_top);

    spawn_local(async move {
        for delay in [0, 16, 48] {
            if delay > 0 {
                delay_ms(delay).await;
            }
            restore_scroll_top(&container, desired_scroll_top);
        }
    });
}

fn restore_scroll_top(container: &HtmlElement, desired_scroll_top: i32) {
    if container.scroll_top() != desired_scroll_top {
        container.set_scroll_top(desired_scroll_top);
    }
}

fn set_game_grid_selected_index(container: &HtmlElement, next_index: usize) {
    let _ = container.set_attribute("data-nav-selected-index", &next_index.to_string());
    if let Ok(event) = web_sys::Event::new("lunchbox-grid-select") {
        let _ = container.dispatch_event(&event);
    }
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
    let current_center_x = current_rect.left() + current_rect.width() / 2.0;
    let current_center_y = current_rect.top() + current_rect.height() / 2.0;

    candidates
        .iter()
        .filter(|candidate| !same_element(candidate, current))
        .filter_map(|candidate| {
            let rect = candidate.get_bounding_client_rect();
            directional_score(current_center_x, current_center_y, &rect, action)
                .map(|score| (score, candidate.clone()))
        })
        .min_by(|(score_a, _), (score_b, _)| {
            score_a
                .overlaps_ray
                .cmp(&score_b.overlaps_ray)
                .reverse()
                .then_with(|| {
                    score_a
                        .primary_distance
                        .partial_cmp(&score_b.primary_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    score_a
                        .perpendicular_distance
                        .partial_cmp(&score_b.perpendicular_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    score_a
                        .center_distance
                        .partial_cmp(&score_b.center_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .map(|(_, candidate)| candidate)
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

struct DirectionalScore {
    overlaps_ray: bool,
    primary_distance: f64,
    perpendicular_distance: f64,
    center_distance: f64,
}

fn directional_score(
    current_center_x: f64,
    current_center_y: f64,
    candidate: &web_sys::DomRect,
    action: NavigationAction,
) -> Option<DirectionalScore> {
    let candidate_center_x = candidate.left() + candidate.width() / 2.0;
    let candidate_center_y = candidate.top() + candidate.height() / 2.0;
    let center_distance = squared_distance(
        current_center_x,
        current_center_y,
        candidate_center_x,
        candidate_center_y,
    );

    match action {
        NavigationAction::Up if candidate_center_y < current_center_y => {
            let overlaps_ray =
                candidate.left() <= current_center_x && current_center_x <= candidate.right();
            let perpendicular_distance =
                distance_from_value_to_span(current_center_x, candidate.left(), candidate.right());
            let primary_distance = if candidate.bottom() <= current_center_y {
                current_center_y - candidate.bottom()
            } else {
                0.0
            };
            Some(DirectionalScore {
                overlaps_ray,
                primary_distance,
                perpendicular_distance,
                center_distance,
            })
        }
        NavigationAction::Down if candidate_center_y > current_center_y => {
            let overlaps_ray =
                candidate.left() <= current_center_x && current_center_x <= candidate.right();
            let perpendicular_distance =
                distance_from_value_to_span(current_center_x, candidate.left(), candidate.right());
            let primary_distance = if candidate.top() >= current_center_y {
                candidate.top() - current_center_y
            } else {
                0.0
            };
            Some(DirectionalScore {
                overlaps_ray,
                primary_distance,
                perpendicular_distance,
                center_distance,
            })
        }
        NavigationAction::Left if candidate_center_x < current_center_x => {
            let overlaps_ray =
                candidate.top() <= current_center_y && current_center_y <= candidate.bottom();
            let perpendicular_distance =
                distance_from_value_to_span(current_center_y, candidate.top(), candidate.bottom());
            let primary_distance = if candidate.right() <= current_center_x {
                current_center_x - candidate.right()
            } else {
                0.0
            };
            Some(DirectionalScore {
                overlaps_ray,
                primary_distance,
                perpendicular_distance,
                center_distance,
            })
        }
        NavigationAction::Right if candidate_center_x > current_center_x => {
            let overlaps_ray =
                candidate.top() <= current_center_y && current_center_y <= candidate.bottom();
            let perpendicular_distance =
                distance_from_value_to_span(current_center_y, candidate.top(), candidate.bottom());
            let primary_distance = if candidate.left() >= current_center_x {
                candidate.left() - current_center_x
            } else {
                0.0
            };
            Some(DirectionalScore {
                overlaps_ray,
                primary_distance,
                perpendicular_distance,
                center_distance,
            })
        }
        _ => None,
    }
}

fn distance_from_value_to_span(value: f64, start: f64, end: f64) -> f64 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.0
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
        .find(|candidate| {
            candidate.get_attribute("data-nav-kind").as_deref() == Some("game-grid")
                || candidate.get_attribute("data-nav-kind").as_deref() == Some("game-item")
        })
    {
        return focus_candidate(game_item);
    }

    candidates.first().map(focus_candidate).unwrap_or(false)
}

fn focus_candidate(candidate: &HtmlElement) -> bool {
    let nav_kind = candidate.get_attribute("data-nav-kind");
    if nav_kind.as_deref() != Some("game-item") && nav_kind.as_deref() != Some("game-grid") {
        candidate.scroll_into_view();
    }
    focus_without_scroll(candidate).is_ok()
}

fn activate_game_grid(container: &HtmlElement) -> bool {
    let selected_index = parse_usize_attr(container, "data-nav-selected-index").unwrap_or(0);
    let selector = format!(r#"[data-game-index="{}"]"#, selected_index);
    let container_element: Element = container.clone().unchecked_into();
    let Some(target) = query_selector(&container_element, &selector).and_then(html_element_from)
    else {
        return false;
    };

    target.click();
    true
}

fn focus_without_scroll(element: &HtmlElement) -> Result<(), wasm_bindgen::JsValue> {
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        options.as_ref(),
        &wasm_bindgen::JsValue::from_str("preventScroll"),
        &wasm_bindgen::JsValue::TRUE,
    );

    match js_sys::Reflect::get(element.as_ref(), &wasm_bindgen::JsValue::from_str("focus"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
    {
        Some(focus_fn) => {
            focus_fn.call1(element.as_ref(), options.as_ref())?;
            Ok(())
        }
        None => element.focus(),
    }
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
