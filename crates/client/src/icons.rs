use crate::editor::Tool;
use yew::prelude::*;

pub fn tool_icon(t: Tool) -> Html {
    let path = match t {
        Tool::Select => {
            html! { <path d="M3 3l7 18 2.5-7.5L20 11 3 3z" fill="currentColor" stroke="none" /> }
        }
        Tool::Rectangle => {
            html! { <rect x="4" y="6" width="16" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" /> }
        }
        Tool::Ellipse => {
            html! { <ellipse cx="12" cy="12" rx="8" ry="6" fill="none" stroke="currentColor" stroke-width="1.75" /> }
        }
        Tool::Line => {
            html! { <line x1="5" y1="18" x2="19" y2="6" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" /> }
        }
        Tool::Arrow => {
            html! {
                <>
                    <line x1="4" y1="19" x2="15" y2="8" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
                    <path d="M15 5l5 5-5 5 2-5z" fill="currentColor" stroke="none" />
                </>
            }
        }
        Tool::Freedraw => {
            html! {
                <path d="M3 16c4-6 7-9 11-7s5 2 7-3" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" />
            }
        }
        Tool::Text => {
            html! {
                <>
                    <path d="M8 6v12M16 6v12M8 12h8" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
                </>
            }
        }
        Tool::Image => {
            html! {
                <>
                    <rect x="4" y="5" width="16" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.75" />
                    <circle cx="9" cy="10" r="1.5" fill="currentColor" />
                    <path d="M4 16l5-5 4 4 3-3 4 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
                </>
            }
        }
        Tool::Eraser => {
            html! {
                <>
                    <path d="M7 17l8-8 3 3-8 8H7v-3z" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linejoin="round" />
                    <path d="M13 6l3 3" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
                </>
            }
        }
    };
    html! {
        <svg class="tool-icon" viewBox="0 0 24 24" aria-hidden="true">
            { path }
        </svg>
    }
}

pub fn tool_label(t: Tool) -> &'static str {
    match t {
        Tool::Select => "Selection",
        Tool::Rectangle => "Rectangle",
        Tool::Ellipse => "Ellipse",
        Tool::Line => "Line",
        Tool::Arrow => "Arrow",
        Tool::Freedraw => "Draw",
        Tool::Text => "Text",
        Tool::Image => "Image",
        Tool::Eraser => "Eraser",
    }
}

pub fn tool_shortcut(t: Tool) -> u8 {
    match t {
        Tool::Select => 1,
        Tool::Rectangle => 2,
        Tool::Ellipse => 3,
        Tool::Line => 4,
        Tool::Arrow => 5,
        Tool::Freedraw => 6,
        Tool::Text => 7,
        Tool::Image => 8,
        Tool::Eraser => 9,
    }
}
