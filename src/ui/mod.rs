//! User interface module

pub mod animation;
pub mod app;
pub mod components;
pub mod dependency_view;
pub mod inspector;
pub mod search;
pub mod splash;
pub mod theme;

pub use animation::{AnimationState, Easing, SmoothScroll};
pub use app::{tabs_rect_for_area, Focus, Tab, VizierUi};
pub use dependency_view::DependencyView;
pub use inspector::InspectorPanel;
pub use search::{
    filter_candidates, CandidateKind, CompletionCandidate, SearchBar, SearchCompletion,
};
