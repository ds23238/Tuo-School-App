# TSA-Sync

Desktop-App für Moodle und WebUntis mit Svelte 5, Tauri 2 und Rust.

## Hybrid-Oberfläche

Die Sidebar, die Heute-Übersicht, der eigene Stundenplan einschließlich
Hausaufgaben und Prüfungen, Einstellungen und Datei-Synchronisierung bleiben
lokal. Die Kursübersicht bleibt ebenfalls lokal, damit sich Kurse für den Sync
auswählen lassen. Beim Öffnen eines Kurses zeigt die App die Moodle-Originalseite
im Inhaltsbereich. Unter Mitteilungen öffnen sich die Originalseiten für
Moodle-Chats, Benachrichtigungen, das optionale Mail-Plugin und WebUntis.

Die Leiste über der Webseite bietet Zurück/Vorwärts, Neuladen, Öffnen im
Systembrowser und den Wechsel zur **lokalen Ansicht**. Bei Kursen bleiben
Synchronisieren und das Öffnen des lokalen Kursordners erreichbar. Die bisherige
Kurs- und Nachrichtenoberfläche bleibt als Rückfall verfügbar, einschließlich
der bereits vorhandenen Offline-Caches. Originalseiten selbst benötigen eine
Netzverbindung.

### Anmeldung

Moodle verwendet den vorhandenen Mobile-Autologin, wenn die Instanz einen
Private-Token und einen Autologin-Schlüssel bereitstellt. WebUntis bekommt beim
Erzeugen seiner Webansicht die Sitzungscookies aus dem Backend. Falls eine Schule
eine zusätzliche Anmeldung verlangt, ist die Anmeldung direkt auf der
Originalseite möglich. Eine Anmeldung nur auf der Webseite ersetzt keine
fehlende API-Anmeldung für Sync oder Stundenplan. Schulabhängige SSO- und
MFA-Abläufe müssen an der jeweiligen Instanz überprüft werden.

Moodle und WebUntis verwenden getrennte Browserprofile. Beim Entfernen eines
Kontos oder einem erneuten API-Login werden die zugehörigen Browserfenster
geschlossen und die Browserdaten zur Löschung angewiesen. Der nächste Login
verwendet zusätzlich einen neuen Profilordner, damit die asynchrone Bereinigung
der Browserengine keine alte Sitzung wiederherstellen kann. Lokale Dateien im
Sync-Ordner bleiben wie bisher bestehen.

### Moodle-Dateien

Bekannte Moodle-Dateilinks werden bei Navigationen, neuen Fenstern und
Download-Anfragen gegen den lokalen Sync-Index geprüft. Unterstützt sind
`pluginfile.php`, `webservice/pluginfile.php`, Moodles `?file=`-Variante und
`mod/resource/view.php?id=…`, sofern die Ressource eindeutig eine Datei enthält.
Download- und Token-Parameter sowie die Revisionsnummer ändern die Zuordnung
nicht. Herkunft, Moodle-Kontext und vollständiger Dateipfad müssen passen;
Dateinamen allein werden niemals zugeordnet.

Eine vorhandene lokale Kopie öffnet sich mit dem Standardprogramm. Eine fehlende
Kopie wird über den bestehenden Sync-Download geladen. Lokal geänderte Dateien
werden beim Öffnen nicht überschrieben. Neue Online-Versionen lassen sich wie
bisher in der lokalen Ansicht ausdrücklich übernehmen. Beim Öffnen eines Kurses
wird sein Datei-Index aktualisiert, auch wenn der Kurs nicht für den automatischen
Sync ausgewählt wurde.

Nicht zuordenbare Downloads werden getrennt unter `Downloads/TSA-Sync` mit einem
eindeutigen Namen gespeichert. Die Leiste der Webansicht öffnet diesen Ordner.
WebUntis-Dateien verwenden normale Browserdownloads.

### Technische Trennung

Die Webseiten laufen in nativen Tauri-Child-WebViews, nicht in iframes und nicht
als kopiertes HTML im lokalen DOM. Sie erhalten keine Tauri-Capabilities für die
lokale Anwendung. Die vorhandene Capability gilt ausschließlich für die lokale
WebView `main`. Browser-Steuerkommandos prüfen zusätzlich die aufrufende
WebView. Web-Popups teilen die Sitzung ihres ursprünglichen Browsers, aber keine
lokalen App-Rechte. Webseiten dürfen HTTP(S)-Weiterleitungen für Identitätsanbieter
verwenden; lokale Protokolle und lokale Tauri-Endpunkte sind als Navigation gesperrt.

Tauri benötigt für mehrere WebViews derzeit das Feature `unstable`. Die
bestehenden CI-Ziele sind Linux und Windows. Ein echter Schul-Login und die
jeweilige Plattform-WebView sollten vor einer Veröffentlichung zusätzlich manuell
geprüft werden.

## Entwicklung

Voraussetzungen: Node.js 22.12+ und ein aktuelles Rust-Stable. Unter Debian/Ubuntu
werden außerdem die Tauri-Systembibliotheken benötigt:

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev libdbus-1-dev
npm ci
npm run dev
```

Die Rust-Bibliotheken liegen unter `crates/core`, `crates/moodle` und
`crates/untis`; die Desktop-Anbindung unter `src-tauri`, die Oberfläche unter `ui`.

## Prüfungen

```sh
npm run check
npm -w ui run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npx playwright install chromium
npm -w ui test
```

Die Playwright-Tests prüfen die Oberfläche mit einer simulierten Tauri-Schnittstelle:
Kursnavigation, Größe beim Einklappen der Sidebar, Dienstwechsel, Browseraktionen
und den Rückfall bei Browserfehlern. Sie ersetzen keinen Test der nativen
WebView. Bei vorhandenem System-Chromium kann dessen Pfad mit
`PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` gesetzt werden.

Die Rust-Tests prüfen unter anderem die eindeutige Zuordnung von Dateilinks,
abweichende Herkunft, mehrdeutige Ressourcen, Sitzungs-Cookies und die bestehende
Dateiversionierung. Tests gegen echte Moodle-Instanzen und den echten
Systemschlüsselbund sind standardmäßig ignoriert.

Ergebnisse und echte Screenshots des Moodle-Demoserver-Tests stehen im
[Testbericht zur Hybrid-Oberfläche](docs/hybrid-validation.md).
