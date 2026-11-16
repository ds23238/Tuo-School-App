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
    let layout = if let Ok(layout) = parent.clone().downcast::<gtk::Layout>() {
        layout
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
            let layout = gtk::Layout::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
            // GtkFixed sizes children from their natural requisition, which
            // WebKit can retain from the previous, wider viewport. A Layout
            // keeps those requisitions out of the parent's preferred size.
            layout.connect_size_allocate(|layout, allocation| {
                for child in layout.children() {
                    child.size_allocate(&gtk::Allocation::new(
                        0,
                        0,
                        allocation.width(),
                        allocation.height(),
                    ));
                }
            });
            layout.set_halign(gtk::Align::Start);
            layout.set_valign(gtk::Align::Start);
            overlay.add_overlay(&layout);
            overlay.show();
            overlay
        };
        let layout = overlay
            .children()
            .into_iter()
            .find_map(|child| child.downcast::<gtk::Layout>().ok())
            .ok_or_else(|| CommandError::new("browser", "GTK-Browserfläche fehlt"))?;
        vbox.remove(view);
        layout.put(view, 0, 0);
        layout
    };
    view.set_size_request(bounds.width.round() as i32, bounds.height.round() as i32);
    // The overlay window must cover only the browser viewport. A full-window
    // transparent container intercepts pointer events intended for the sidebar.
    layout.set_margin_start(bounds.x.round() as i32);
    layout.set_margin_top(bounds.y.round() as i32);
    layout.set_size_request(bounds.width.round() as i32, bounds.height.round() as i32);
    layout.set_size(bounds.width.round() as u32, bounds.height.round() as u32);
    layout.move_(view, 0, 0);
    // Update the engine viewport immediately as well as on later GTK layouts.
    view.size_allocate(&gtk::Allocation::new(
        0,
        0,
        bounds.width.round() as i32,
        bounds.height.round() as i32,
    ));
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

/// Hide the overlay itself when all browser children are hidden; otherwise
/// its input window would block the local Settings/course page underneath.
pub async fn visibility(view: &Webview, visible: bool) -> Result<()> {
    let (send, receive) = tokio::sync::oneshot::channel();
    view.with_webview(move |platform| {
        let inner = platform.inner();
        if visible {
            inner.show();
        } else {
            inner.hide();
        }
        if let Some(layout) = inner
            .parent()
            .and_then(|parent| parent.downcast::<gtk::Layout>().ok())
        {
            if visible {
                layout.show();
            } else if !layout.children().iter().any(|child| child.is_visible()) {
                layout.hide();
            }
        }
        let _ = send.send(());
    })?;
    receive
        .await
        .map_err(|e| CommandError::new("browser", e.to_string()))?;
    Ok(())
}
