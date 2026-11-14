//! GTK's default VBox ignores child-WebView coordinates. Keep the local view
//! full size and position remote views in an overlay on top of it instead.
use gtk::prelude::*;
use tauri::Webview;

use super::Bounds;
use crate::error::{CommandError, Result};

fn place(view: &webkit2gtk::WebView, bounds: Bounds) -> Result<()> {
    let parent = view
        .parent()
        .ok_or_else(|| CommandError::new("browser", "WebView ohne GTK-Container"))?;
    let fixed = if let Ok(fixed) = parent.clone().downcast::<gtk::Fixed>() {
        fixed
    } else {
        let vbox = parent
            .downcast::<gtk::Box>()
            .map_err(|_| CommandError::new("browser", "Unbekannter GTK-Container"))?;
        // Reuse one overlay for both services. Only the local WebView remains
        // the main child; remote views are positioned independently of it.
        let existing = vbox
            .children()
            .into_iter()
            .find_map(|child| child.downcast::<gtk::Overlay>().ok());
        let overlay = if let Some(overlay) = existing {
            overlay
        } else {
            let local = vbox
                .children()
                .into_iter()
                .find(|child| child != view && child.is::<webkit2gtk::WebView>())
                .ok_or_else(|| CommandError::new("browser", "Lokale GTK-WebView fehlt"))?;
            vbox.remove(&local);
            let overlay = gtk::Overlay::new();
            overlay.add(&local);
            overlay.set_hexpand(true);
            overlay.set_vexpand(true);
            vbox.pack_start(&overlay, true, true, 0);
            let fixed = gtk::Fixed::new();
            fixed.set_hexpand(true);
            fixed.set_vexpand(true);
            overlay.add_overlay(&fixed);
            overlay.show();
            fixed.show();
            overlay
        };
        let fixed = overlay
            .children()
            .into_iter()
            .find_map(|child| child.downcast::<gtk::Fixed>().ok())
            .ok_or_else(|| CommandError::new("browser", "GTK-Browserfläche fehlt"))?;
        vbox.remove(view);
        fixed.put(view, bounds.x.round() as i32, bounds.y.round() as i32);
        fixed
    };
    view.set_size_request(bounds.width.round() as i32, bounds.height.round() as i32);
    fixed.move_(view, bounds.x.round() as i32, bounds.y.round() as i32);
    Ok(())
}

pub async fn position(view: &Webview, bounds: Bounds) -> Result<()> {
    let (send, receive) = tokio::sync::oneshot::channel();
    view.with_webview(move |platform| {
        let _ = send.send(place(&platform.inner(), bounds));
    })?;
    receive
        .await
        .map_err(|e| CommandError::new("browser", e.to_string()))?
}
