//! Menu entries and their pages (`docs/extending.md`, "Menu entries and
//! pages"): a project's buttons on the main menu, each opening a page of
//! captions, toggles, choices and buttons drawn like the settings pages
//! (UI-MENU-4.1). Without entries the main menu is the original's.

use bevy::prelude::*;

/// A main menu entry: the button's label and the page it opens.
pub struct MenuEntry {
    /// The button's label, in `font_big_white` like the original rows
    /// (which has capitals only); also the page's title.
    pub label: String,
    pub page: Box<dyn MenuPage>,
}

/// What a page shows: its widgets from the top, asked for every frame the
/// page is on screen, read from the app's world (a project's resources).
/// A closure `Fn(&World) -> Vec<PageWidget>` is a page.
pub trait MenuPage: Send + Sync + 'static {
    fn widgets(&self, world: &World) -> Vec<PageWidget>;
}

impl<F> MenuPage for F
where
    F: Fn(&World) -> Vec<PageWidget> + Send + Sync + 'static,
{
    fn widgets(&self, world: &World) -> Vec<PageWidget> {
        self(world)
    }
}

/// Called with the app's world and the new value when the player changes
/// a widget; after the frame's menu actions, before the page is asked for
/// its widgets again.
pub type OnChange<T> = Box<dyn FnOnce(&mut World, T) + Send + Sync>;

/// Called with the app's world when the player clicks a button.
pub type OnClick = Box<dyn FnOnce(&mut World) + Send + Sync>;

/// A row of a page, laid out like the settings pages (UI-MENU-4.1): from
/// (168, 72) of the dialog stepping 32 px, the label 144 px left of the
/// row in the headings' font and colour, the chosen value yellow.
pub enum PageWidget {
    /// A row with its text alone.
    Caption(String),
    /// Off and On, as INFORMATION WINDOW.
    Toggle {
        label: String,
        value: bool,
        on_change: OnChange<bool>,
    },
    /// The options three to a line, 100 px apart, lines 24 px apart, as
    /// AUTOSAVE; `index` is the chosen one.
    Choice {
        label: String,
        options: Vec<String>,
        index: usize,
        on_change: OnChange<usize>,
    },
    /// The dialogs' 152 x 22 button at the row's x.
    Button { label: String, on_click: OnClick },
    /// Hook W22: the rows after it start the page's next part (reached by
    /// the page switch at the page's foot). Rows that do not fit above the
    /// switch go on to the next part by themselves.
    PageBreak,
}

/// A row as drawn: a [`PageWidget`] without its callback.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Row {
    Caption(String),
    Toggle {
        label: String,
        value: bool,
    },
    Choice {
        label: String,
        options: Vec<String>,
        index: usize,
    },
    Button(String),
    PageBreak,
}

/// What the player did to a page's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageInput {
    Toggle(bool),
    Choose(usize),
    Click,
}

impl PageWidget {
    fn row(&self) -> Row {
        match self {
            Self::Caption(text) => Row::Caption(text.clone()),
            Self::Toggle { label, value, .. } => Row::Toggle {
                label: label.clone(),
                value: *value,
            },
            Self::Choice {
                label,
                options,
                index,
                ..
            } => Row::Choice {
                label: label.clone(),
                options: options.clone(),
                index: *index,
            },
            Self::Button { label, .. } => Row::Button(label.clone()),
            Self::PageBreak => Row::PageBreak,
        }
    }

    /// Run the callback the input is for, if it is this widget's.
    fn call(self, world: &mut World, input: PageInput) {
        match (self, input) {
            (Self::Toggle { on_change, .. }, PageInput::Toggle(on)) => on_change(world, on),
            (Self::Choice { on_change, .. }, PageInput::Choose(n)) => on_change(world, n),
            (Self::Button { on_click, .. }, PageInput::Click) => on_click(world),
            _ => {}
        }
    }
}

/// The pages of the entries, and the widgets of the page on screen with
/// their callbacks (taken when used).
#[derive(Resource)]
pub(crate) struct Pages {
    pages: Vec<Box<dyn MenuPage>>,
    shown: Vec<Option<PageWidget>>,
}

/// The main menu has two free rows (UI-MENU-1.2: between CREDITS and
/// EXIT); entries past them are not shown.
pub(crate) const MENU_ROWS: usize = 2;

/// The entries' labels for the main menu, and their pages as a resource.
pub(crate) fn split(entries: Vec<MenuEntry>) -> (Vec<String>, Pages) {
    if entries.len() > MENU_ROWS {
        warn!(
            "the main menu has room for {MENU_ROWS} entries; {} are left out",
            entries[MENU_ROWS..]
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let (labels, pages) = entries
        .into_iter()
        .take(MENU_ROWS)
        .map(|e| (e.label, e.page))
        .unzip();
    (
        labels,
        Pages {
            pages,
            shown: Vec::new(),
        },
    )
}

/// After the frame's menu actions: the callbacks of the rows the player
/// changed, then the rows of the page on screen for the next frame
/// (`MenuState::page`). Added only when there are entries.
pub(crate) fn run_page(world: &mut World) {
    let mut menu = world.resource_mut::<crate::menus::menu::MenuState>();
    let inputs = std::mem::take(&mut menu.page_input);
    world.resource_scope(|world, mut pages: Mut<Pages>| {
        for (n, input) in inputs {
            if let Some(widget) = pages.shown.get_mut(n).and_then(Option::take) {
                widget.call(world, input);
            }
        }
        let screen = world.resource::<crate::menus::menu::MenuState>().screen;
        let widgets = match screen {
            crate::menus::menu::Screen::Extension(i) => {
                pages.pages.get(i).map(|p| p.widgets(world))
            }
            _ => None,
        }
        .unwrap_or_default();
        world.resource_mut::<crate::menus::menu::MenuState>().page =
            widgets.iter().map(PageWidget::row).collect();
        pages.shown = widgets.into_iter().map(Some).collect();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::menu::{MenuState, Screen};
    use bevy::ecs::system::RunSystemOnce;

    #[derive(Resource, Default)]
    struct Switch(bool);

    fn page() -> MenuEntry {
        MenuEntry {
            label: "FRONTIER".into(),
            page: Box::new(|world: &World| {
                let on = world.resource::<Switch>().0;
                vec![
                    PageWidget::Caption("Mods".into()),
                    PageWidget::Toggle {
                        label: "Follow camera".into(),
                        value: on,
                        on_change: Box::new(|world, on| world.resource_mut::<Switch>().0 = on),
                    },
                ]
            }),
        }
    }

    // spec: none (the menu hook: a page's rows and its callbacks)
    #[test]
    fn a_page_shows_its_rows_and_runs_its_callbacks() {
        let (labels, pages) = split(vec![page()]);
        assert_eq!(labels, ["FRONTIER"]);
        let mut world = World::new();
        world.init_resource::<Switch>();
        world.insert_resource(pages);
        world.insert_resource(MenuState {
            screen: Screen::Extension(0),
            ..MenuState::default()
        });
        world.run_system_once(run_page).unwrap();
        let rows = world.resource::<MenuState>().page.clone();
        assert_eq!(
            rows,
            [
                Row::Caption("Mods".into()),
                Row::Toggle {
                    label: "Follow camera".into(),
                    value: false
                }
            ]
        );
        world
            .resource_mut::<MenuState>()
            .page_input
            .push((1, PageInput::Toggle(true)));
        world.run_system_once(run_page).unwrap();
        assert!(world.resource::<Switch>().0);
        assert!(matches!(
            world.resource::<MenuState>().page[1],
            Row::Toggle { value: true, .. }
        ));
        // Off the page, no rows.
        world.resource_mut::<MenuState>().screen = Screen::Main;
        world.run_system_once(run_page).unwrap();
        assert!(world.resource::<MenuState>().page.is_empty());
    }

    // spec: UI-MENU-1.2 (two free rows between CREDITS and EXIT)
    #[test]
    fn two_entries_fit() {
        let (labels, _) = split(vec![page(), page(), page()]);
        assert_eq!(labels.len(), MENU_ROWS);
    }
}
