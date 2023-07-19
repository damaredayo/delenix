use gtk::prelude::*;
use gtk::{Label, Window, WindowType};

use std::error::Error;
use std::time::Duration;

use crate::util;

// TODO: make notifications not block the main thread, for now, we'll keep the code here but not use it

pub fn show_notification(message: &str) -> Result<(), Box<dyn Error>> {
    let window = Window::new(WindowType::Popup);
    window.set_title("Notification");
    window.set_decorated(false);
    window.set_keep_above(true);

    let (screen_width, screen_height) = util::get_monitor_resolution()?;

    let label = Label::new(Some(message));

    let gtk_box = gtk::EventBox::new();

    gtk_box.add(&label);

    // Add click event to open link in default browser if the message is a link
    if message.starts_with("http://") || message.starts_with("https://") {
        let css_provider = gtk::CssProvider::new();
        let css = "label { color: blue; text-decoration: underline; }";
        css_provider.load_from_data(css.as_bytes())?;

        let context = label.style_context();
        context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let url = message.to_string();
        gtk_box.connect_button_press_event(move |_box, event| {
            if event.button() == 1 {
                if let Err(e) = webbrowser::open(&url) {
                    tracing::error!("Failed to open link: {}", e);
                }
            }

            gtk::main_quit();
            Inhibit(false)
        });

        gtk_box.connect_enter_notify_event(|gtk_box, _| {
            gtk_box.window().unwrap().set_cursor(
                gdk::Cursor::for_display(&gdk::Display::default().unwrap(), gdk::CursorType::Hand2)
                    .as_ref(),
            );

            Inhibit(false)
        });

        gtk_box.connect_leave_notify_event(|gtk_box, _| {
            gtk_box.window().unwrap().set_cursor(
                gdk::Cursor::for_display(&gdk::Display::default().unwrap(), gdk::CursorType::Arrow)
                    .as_ref(),
            );

            Inhibit(false)
        });

        gtk_box.add_events(
            gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK
                | gdk::EventMask::BUTTON_PRESS_MASK,
        );
    }

    window.connect_destroy(|_| {
        gtk::main_quit();
    });

    // after 5 seconds, close the notification
    glib::timeout_add(Duration::from_secs(5), move || {
        gtk::main_quit();
        Continue(false)
    });

    // Adjust the size based on the length of the text
    let label_width = label.layout().unwrap().pixel_size().0 as i32;
    let height = 30;
    let padding = 20;

    window.set_default_size(label_width + 10, height);

    let x = screen_width - label_width - padding;
    let y = screen_height - height - padding;
    window.move_(x, y);

    // Add the label to the window
    window.add(&gtk_box);

    // Show the window and start the GTK main event loop
    window.show_all();
    gtk::main();

    Ok(())
}
