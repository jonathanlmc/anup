use crossterm::event::KeyModifiers;

use crate::tui::{self, event};

pub(super) async fn process(
    event: crossterm::event::Event,
    state: &mut tui::AppState,
    panels: &mut tui::AllPanels,
    render_trigger: &tui::RenderTrigger,
) -> event::Result {
    use crossterm::event::Event;

    match event {
        Event::Resize(_, _) => render_trigger.notify_one(),
        Event::Key(key) => {
            if key.is_press()
                && key.modifiers.contains(KeyModifiers::SHIFT)
                && key.code.is_char('Q')
            {
                return event::Result::Quit;
            }

            for panel in panels {
                panel.process_input(key, state, render_trigger);
            }
        }
        _ => (),
    }

    event::Result::Continue
}
