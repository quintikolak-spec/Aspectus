# Personal AI Desktop Dashboard — Projekt-Grundgerüst

Ressourcenschonendes Desktop-Dashboard für Windows/Linux mit KI-gestützter
Zusammenfassung eigener Nachrichtenquellen. Siehe die ursprüngliche
Projektspezifikation für den vollständigen Funktionsumfang.

## Stack & Begründung

- **Tauri 2** (Rust-Backend + native WebView) statt Electron — kein
  gebündeltes Chromium, dadurch deutlich geringerer RAM-/CPU-Verbrauch im
  Leerlauf. Volle Windows- und Linux-Unterstützung, System-Tray und
  Autostart sind first-class Features.
- **Svelte + TypeScript** im Frontend — kein virtueller DOM, kleine Bundles,
  passt zum Performance-Ziel besser als React für ein Dauerlauf-Widget-System.
- **SQLite** (`rusqlite`, bundled) als einzige lokale Datenquelle für
  Einstellungen, Quellen, bereits gesehene Artikel (Dedup-Hashes),
  KI-Zusammenfassungen und einen generischen Key-Value-Cache (Wetter etc.).
- **RSS/Atom bevorzugt** (`feed-rs`), HTML-Fallback (`scraper`) — RSS ist
  ressourcenschonender und zuverlässiger, wie in der Spec gefordert.

## Verzeichnisstruktur

```
src/                     Svelte-Frontend
  lib/
    api.ts               einzige Schnittstelle zu Tauri-Commands
    stores/               settings, theme, update-cycle (Eco-Mode-Timing)
    dashboard/            Grid, Drag&Drop, "Widget hinzufügen"
    widgets/              Clock, Weather, News, System, Calendar, Media, Source
    settings/             Interessen, Modus, Theme, Quellenverwaltung

src-tauri/                Rust-Backend
  src/
    main.rs               Setup: DB, Tray, Autostart, Background-Loop
    commands.rs            Tauri-Commands (die einzige Frontend-Bridge)
    db.rs                  SQLite-Schema
    models.rs              Shared Datenmodelle
    core/
      settings.rs          Laden/Speichern der Konfiguration
      source_manager.rs     Quellen hinzufügen/abrufen/Dedup per Hash
      news_engine.rs        Orchestriert: Fetch → Filter → KI → Speichern
      ai_engine.rs           Lokaler Vorfilter + Batch-KI-Aufruf
      auth.rs                 OpenAI-Token-Storage im OS-Keychain
      system_monitor.rs        CPU/RAM/Disk via sysinfo
      cache.rs                  Generischer TTL-Cache
      update_manager.rs         Hintergrund-Scheduler nach Eco-Mode
      notifications.rs           Benachrichtigung bei sehr wichtigen News
```

## Änderungen nach dem ersten lokalen Testlauf (bei dir auf CachyOS)

Der erste `npm run tauri dev` bei dir kompilierte bereits erfolgreich und
startete das Fenster fast bis zum Ende — nur drei kleine Laufzeitprobleme
kamen zutage, alle behoben:

1. **`tauri.conf.json`**: Das `notification`-Plugin erwartet gar keinen
   Konfigurationswert (auch kein leeres `{}`) — das hat den Start mit
   einem Deserialisierungsfehler abgebrochen. Entfernt.
2. **Deprecation**: `Shell::open` ist zugunsten von `tauri-plugin-opener`
   veraltet — sowohl Rust- (`core/auth.rs`) als auch Frontend-Seite
   (News-/Source-Widget) umgestellt.
3. **Fehlende Capabilities-Datei**: Tauri 2 verlangt eine explizite
   Freigabe, welche Befehle das Frontend aufrufen darf
   (`src-tauri/capabilities/default.json`) — die fehlte komplett im
   ersten Entwurf. Ohne sie wären vermutlich die ersten `invoke()`-Aufrufe
   (z. B. Einstellungen laden) mit einem Permission-Fehler gescheitert.

## Verifizierungsstand

**Frontend (Svelte/TypeScript): getestet und lauffähig.**
- `npm install`, `npm run build` und `npx svelte-check` laufen fehlerfrei
  durch (0 Fehler, 0 Warnungen). Dabei wurden bereits mehrere echte Bugs
  gefunden und behoben: fehlender TS-Preprocessor in `vite.config.ts`,
  ungültige inline `as`-Casts in Svelte-Templates, eine fehlende
  npm-Abhängigkeit (`@tauri-apps/plugin-shell`), ein falscher
  `onMount`-Rückgabewert sowie fehlende Vite-Ambient-Types.

**Backend (Rust/Tauri): nicht kompiliert.**
- In dieser Umgebung ist kein `cargo`/Rust-Toolchain verfügbar, außerdem
  fehlen Tauri-Systemabhängigkeiten (webkit2gtk etc.) und ein Display.
  Der Rust-Code wurde sorgfältig geschrieben und gegen die Tauri-2- und
  OpenAI-API-Dokumentation geprüft, ist aber **nicht** compiler-verifiziert.
  Nächster Schritt: bei dir lokal `npm run tauri dev` ausführen und mir
  die Fehlermeldungen schicken — die beheben wir dann gezielt.

## Setup

Voraussetzungen: Node.js ≥ 18, Rust (stable), sowie die
[Tauri-Systemabhängigkeiten](https://tauri.app/start/prerequisites/) für
dein Betriebssystem (unter Linux z. B. `webkit2gtk`, unter Windows die
WebView2-Runtime, die auf aktuellen Windows-Versionen meist vorinstalliert ist).

```bash
npm install
npm run tauri dev      # Entwicklungsmodus mit Hot Reload
npm run tauri build    # produziert .deb/.AppImage bzw. .msi/.exe
```

## Systemmonitor-Erweiterung (nach deinen Compiler-Fehlern gefixt)

Die von ChatGPT ergänzte `SystemMonitor`-Struct (persistente CPU-Messung,
Pro-Sensor-Temperaturen, GPU via `nvidia-smi`/`amdgpu`-sysfs, Load Average,
Top-Prozesse) hatte drei sysinfo-0.32-API-Abweichungen — behoben:
- `Components::refresh()` nimmt in 0.32 keinen Parameter mehr
- `Component::temperature()` gibt `f32` zurück, kein `Option<f32>`
- `get_system_stats` rief eine freie Funktion statt einer Methode auf der
  Struct auf — jetzt läuft `SystemMonitor` als Tauri-managed State
  (`Mutex<SystemMonitor>`), damit die CPU-Werte wirklich stimmen (siehe
  Kommentar in `system_monitor.rs`: ein neues `System` pro Aufruf meldet
  immer ~0%, weil CPU-Auslastung nur als Delta zwischen zwei Messungen
  Sinn ergibt).

Frontend ergänzt: Gesamtsystem-Auslastung, Temperatur-Chips, GPU-Anzeige
und eine Top-Prozesse-Liste im System-Widget, bei der sich einzelne
Prozesse ausblenden und wieder einblenden lassen (persistiert in der
jeweiligen Widget-Layout-Konfiguration).

## Kritischer Fix: App-weiter Absturz durch doppelte Prozessnamen

Die gemeldeten „toten“ Buttons hatten eine einzige Ursache: die
Prozessliste im System-Widget wurde per Prozessname als Svelte-Key
gerendert (`{#each ... (proc.name)}`). Chromium-basierte Apps (Brave,
Chrome, ...) starten pro Tab/Fenster einen eigenen Prozess, alle mit
demselben Namen — ein doppelter Key lässt Svelte eine Exception werfen,
die die Reaktivität der *gesamten* App lahmlegt, nicht nur der Liste.
Behoben, indem Prozesse serverseitig nach Name gruppiert werden (CPU/RAM
summiert) — das macht Namen strukturell eindeutig und ist nebenbei auch
nützlicher (eine Zeile pro App statt einer pro Tab, wie im
KDE-Systemmonitor).

## Systemmonitor: Übersichtlichkeit & Genauigkeit (zweite Runde)

- **Prozess-Prozentzahlen normalisiert**: `sysinfo` meldet Prozess-CPU
  standardmäßig relativ zu *einem* Kern (100% = 1 Kern voll ausgelastet) —
  ein Multi-Thread-Prozess kann dadurch >100% anzeigen (daher z. B.
  „node“-Prozesse mit über 100%). Jetzt durch die Kernanzahl geteilt, damit
  es sich wie ein gewohnter Task-Manager liest (0–100% vom Gesamtsystem).
- **Temperatur-Sensoren kategorisiert**: statt roher Labels wie
  `gigabyte_wmi temp3` zeigt die App jetzt `CPU`, `GPU`, `Mainboard`,
  `Datenträger` oder `Sonstige` — pro Kategorie der höchste gemessene Wert.
  Zuordnung per Label-Heuristik in `categorize_temp_label()`
  (`system_monitor.rs`); bei ungewöhnlicher Hardware kann die Liste dort
  erweitert werden.
- **CPU-Genauigkeit**: `SystemMonitor::new()` nimmt jetzt zwei Messungen
  mit 250ms Abstand direkt beim Start, bevor die UI die erste Zahl sieht —
  vorher basierte die allererste Anzeige auf einem einzelnen, nicht
  aussagekräftigen Sample.
- **Übersicht**: CPU/RAM/GPU/System jetzt als runde Ringe (ähnlich dem
  KDE-Systemmonitor-Look) statt reiner Zahlen.

## Bekanntes offenes Problem: UI-Buttons reagieren nicht

Settings-, „+“- und Widget-Steuerungs-Buttons wurden gemeldet als nicht
klickbar. Der Code dafür (`App.svelte`, `Dashboard.svelte`,
`WidgetContainer.svelte`) wurde geprüft und zeigt keinen offensichtlichen
Fehler — ohne eine echte Browser-Konsolen-Fehlermeldung aus der laufenden
App wäre jeder Fix hier reines Raten. Nächster Schritt: WebKit-Inspector
öffnen (Rechtsklick irgendwo im App-Fenster → „Element untersuchen“ /
„Inspect Element“ → Tab „Console“), einen kaputten Button klicken, und die
dort rot erscheinende Fehlermeldung kopieren.

## OpenAI-Integration (Stand: aktuelle OpenAI-Doku geprüft)

Die Assistants API wurde am 26.08.2026 offiziell abgeschaltet — die
aktuelle Empfehlung ist die **Responses API** kombiniert mit der
**Conversations API** für persistenten Zustand. Entsprechend umgesetzt:

- **`core/ai_engine.rs`**: ruft `POST /v1/responses` mit `conversation:
  <id>` auf (statt zustandsloser Chat-Completions). Die Conversation-ID
  wird einmalig über `POST /v1/conversations` erzeugt und dauerhaft im
  lokalen Cache gespeichert (§19 „persistente Projekt-/Kontextstruktur“) —
  Conversation-Items unterliegen laut OpenAI-Doku keiner 30-Tage-TTL, im
  Gegensatz zu einzelnen Responses. Für die JSON-Ausgabe wird Structured
  Outputs (`text.format: json_schema, strict: true`) genutzt statt
  freihändigem JSON-Prompting.
- **Modell**: `gpt-6-astra` als aktuelles Flaggschiff-Beispiel aus der
  OpenAI-Doku. Modellnamen ändern sich — bei „unknown model“-Fehlern in
  `https://developers.openai.com/api/docs/models` nachsehen.
- **`core/auth.rs`** implementiert zwei Wege, wie in §18 gefordert („kein
  Passwort speichern“, „offiziell vorgesehene Authentifizierung“):
  1. **API-Key** (`sign_in_with_api_key`) — funktioniert sofort für jeden
     Entwickler über platform.openai.com/api-keys, nur der Key landet im
     OS-Keychain. Das ist aktuell der pragmatische Standardweg.
  2. **„Sign in with ChatGPT“** (`sign_in_oauth`) — ein echter PKCE-
     OAuth-Flow gegen `auth.openai.com`, mit lokalem Loopback-Server für
     den Redirect (gleiches Muster wie Codex CLI). **Wichtig:** Dieser
     Flow benötigt laut aktueller Recherche eine bei OpenAI registrierte
     `client_id` (Entwickler-Interessensformular) — ohne die bricht die
     Funktion mit einer klaren Fehlermeldung ab, statt fehlzuschlagen.
     Sobald eine `client_id` vorliegt, einfach die Konstante `CLIENT_ID`
     in `core/auth.rs` ersetzen.
- Das Settings-Panel im Frontend bietet beide Wege an (API-Key-Feld +
  „Mit ChatGPT-Konto anmelden“-Button).

Da sich OpenAI-APIs weiterentwickeln, lohnt sich vor dem produktiven
Einsatz ein erneuter Blick auf `developers.openai.com/api/docs`.

## Was sonst noch offen ist

1. **HTML-Extraktion** (`fetch_html` in `source_manager.rs`) ist bewusst
   simpel gehalten (Selektor-Heuristik). Für produktionsreife Extraktion
   lohnt sich eine Readability-ähnliche Bibliothek oder pro-Seite-Regeln.
2. **GPU/Akku/Temperaturen** im Systemmonitor sind Platzhalter — `sysinfo`
   deckt das nicht plattformübergreifend ab; ggf. `nvml-wrapper` (NVIDIA)
   und `starship-battery` ergänzen.
3. **Icons**: `src-tauri/icons/` ist leer — vor dem ersten Build echte
   Icon-Dateien (icon.png, .ico, .icns) ergänzen, sonst schlägt der Build fehl.
4. Dieses Grundgerüst wurde nicht in einer Build-Umgebung mit GUI/Tauri-
   Systemabhängigkeiten kompiliert — vor dem ersten `npm run tauri dev`
   lohnt ein Blick auf Compiler-Fehler, insbesondere bei den Tauri-2-
   Plugin-APIs, da sich deren genaue Signaturen zwischen Minor-Versionen
   noch ändern.
