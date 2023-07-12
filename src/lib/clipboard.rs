use gdk_pixbuf::{traits::PixbufLoaderExt, PixbufLoader};

// i would like to take this opportunity to say fuck x11 and thank you gtk

pub fn copy_png_to_clipboard(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let atom = gdk::Atom::intern("CLIPBOARD");
    let clipboard = gtk::Clipboard::get(&atom);

    let loader = PixbufLoader::new();
    loader.write(data)?;
    loader.close()?;

    if let Some(pixbuf) = loader.pixbuf() {
        clipboard.set_image(&pixbuf);

        clipboard.store();

        return Ok(());
    }

    Err("Failed to get Pixbuf from loader")?
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let atom = gdk::Atom::intern("CLIPBOARD");
    let clipboard = gtk::Clipboard::get(&atom);

    clipboard.set_text(text);

    clipboard.store();

    Ok(())
}
