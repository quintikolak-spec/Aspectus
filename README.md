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

## Windows-Build über GitHub Actions (ohne eigenen Windows-Rechner)

`.github/workflows/build-windows.yml` baut die App auf einem von GitHub
bereitgestellten Windows-Runner — WebView2 und die MSVC-Build-Tools sind
dort bereits vorinstalliert, anders als die `webkit2gtk`-Abhängigkeiten,
die lokal unter CachyOS nötig waren.

**Einmalige Einrichtung:**
1. Projekt in ein GitHub-Repository pushen (falls noch nicht vorhanden:
   auf github.com ein neues, leeres Repo anlegen, dann lokal
   `git init && git add -A && git commit -m "init" && git remote add origin <repo-url> && git push -u origin main`).
2. Auf GitHub zum Tab **„Actions"** wechseln.

**Zum Bauen:**
- Läuft automatisch bei jedem Push auf `main`, **oder**
- manuell: Actions-Tab → „Build Windows" → „Run workflow".

**Ergebnis abholen:** Sobald der Lauf grün ist, unten auf der
Zusammenfassungsseite unter „Artifacts" auf `windows-installers` klicken —
enthält sowohl eine `.msi`- als auch eine `.exe`-Installationsdatei zum
Download. Kein GitHub-Token, kein Release, kein Tag nötig.

## Wetter-Widget war oben abgeschnitten

`justify-content: center` im Wetter-Widget zentrierte den Inhalt vertikal
— sobald durch die neuen Zusatzdaten (Details, Vorschau) mehr Inhalt da
war als Platz in der Box, wurde die obere Hälfte davon nach oben aus dem
sichtbaren Bereich gedrückt (der Scrollcontainer beginnt oben bei
Position 0, nicht mittig). Behoben durch `justify-content: flex-start`.
Zusätzlich hat das Wetter-Widget in den Standardeinstellungen jetzt mehr
Höhe. **Bereits bestehende Installationen** übernehmen diesen neuen
Standardwert nicht automatisch (er gilt nur für frische Installationen);
dort einfach über den „+"-Button am Widget selbst einmal vergrößern.

## Sechste Runde: Regler-Smoothness, Scrollbar-Überlappung, Wetter-Details, Quellen-Filter

- **Lautstärkeregler jetzt smooth**: visuelle Position aktualisiert sich
  sofort beim Ziehen (`on:input`), der eigentliche MPRIS-Aufruf ist per
  80ms-Debounce entkoppelt statt bei jedem Pixel zu feuern — beim
  Loslassen (`on:change`) wird der finale Wert sofort ohne Debounce-Delay
  gesendet.
- **Scrollbar überlappte Widget-Inhalte am rechten Rand — bei allen
  Widgets**: `WidgetContainer.svelte`'s `.content`-Bereich hatte keinen
  Platz für die Scrollbar reserviert, wodurch Regler/Buttons/Selects, die
  bis an den Rand reichten, mit ihr kollidierten. Jetzt mit
  Padding/Margin-Trick behoben, zusätzlich global schlankere
  WebKit-Scrollbars (`::-webkit-scrollbar` in `app.css`).
- **Wetter „zeigt keine genauen Sachen"**: zwei echte Lücken behoben —
  die Mehrtagesvorschau war als leeres Array nie implementiert (jetzt aus
  Open-Meteos `daily`-Feldern gemappt, 4 Tage), und der Ortsname war immer
  nur „Aktueller Standort" (jetzt echte Reverse-Geocoding via Nominatim/
  OpenStreetMap, kostenlos, kein API-Key).
- **News-Quellen-Filter**: bei mehr als einer Quelle erscheint im
  News-Widget ein Dropdown („Alle Quellen" oder eine bestimmte), analog
  zur Player-Auswahl im Medien-Widget, pro Widget gespeichert.

## Fünfte Runde: verschwindende News, Lautstärke-Stottern, Wetter-Details, Player-Auswahl

- **News-Karten verschwanden nach dem zweiten „Jetzt aktualisieren"**:
  Der Fallback-Pfad (KI gerade nicht signiert/verfügbar) zeigte eine Karte
  an, speicherte sie aber nie in der DB — während der zugrunde liegende
  Artikel trotzdem als „bereits gesehen" markiert wurde. Ergebnis: die
  Karte war für immer weg, sobald man nochmal aktualisierte. Jetzt wird
  auch dieser Fallback-Pfad persistiert.
- **Lautstärkeregler stotterte**: löste bei jedem Drag-Tick einen
  MPRIS-Aufruf aus, während der 2-Sekunden-Poll den Regler mitten im
  Ziehen zurücksetzte. Reagiert jetzt nur noch auf Loslassen/Klick
  (`on:change` statt `on:input`), und der Poll pausiert, solange aktiv
  gezogen wird.
- **Dropdown-Pfeile verkleinert** (waren unnötig groß).
- **Wetter erweitert**: gefühlte Temperatur, Luftfeuchtigkeit,
  Windgeschwindigkeit über Open-Meteos `current`-Parameter.
- **Medien-Player-Auswahl**: `core/media.rs` akzeptiert jetzt überall einen
  bevorzugten Player-Namen (z. B. "Spotify", "Brave") statt blind zu
  raten; eine neue `media_list_players`-Abfrage listet alle aktiven
  MPRIS-Player, das Widget zeigt bei mehreren Playern ein Auswahl-Dropdown
  (nur sichtbar, wenn tatsächlich >1 Player aktiv ist), Auswahl wird pro
  Widget gespeichert.

## Compile-Fix: `scraper::Html` ist nicht `Send`

`probe_source` hielt das geparste HTML-Dokument (`scraper::Html`) sowohl
vor als auch nach dem RSS-Discovery-Fetch am Leben — da `scraper::Html`
nicht `Send` ist, macht das die gesamte Async-Funktion nicht `Send`, was
Tauri-Commands zwingend voraussetzen. Behoben, indem alles, was aus dem
Dokument gebraucht wird (Feed-Link *und* HTML-Titel-Fallback), in einem
synchronen Block extrahiert wird, der vor dem nächsten `.await` endet —
das Dokument selbst existiert danach nicht mehr.

## Vierte Runde: Lautstärke, API-Key-Speicherung, Windows-Terminal, RSS-Erkennung

- **Lautstärkeregler** im Medien-Widget ergänzt (MPRIS `Volume`-Property,
  get/set über `media_get_volume`/`media_set_volume`).
- **API-Key-Speicherung**: `sign_in_with_api_key` verifiziert jetzt nach
  dem Schreiben, dass der Wert auch wirklich zurückgelesen werden kann,
  und gibt bei Fehlschlag eine klare Ursache zurück (fehlender/gesperrter
  Schlüsselbund-Dienst wie KWallet/gnome-keyring) statt der bisherigen
  irreführenden Pauschalmeldung. Das Frontend zeigt jetzt den echten
  Fehlertext an. **Wenn der Fehler weiterhin auftritt**: das ist meist ein
  Systemproblem, kein App-Bug — prüfen, ob unter CachyOS ein
  Secret-Service-Provider läuft (z. B. `kwalletd6` bei KDE, oder
  `gnome-keyring-daemon`) und beim Login automatisch entsperrt wird.
- **Windows: Terminal blitzt bei jedem Update auf** — verursacht durch den
  `nvidia-smi`-Unterprozessaufruf für die GPU-Anzeige, den Windows ohne
  explizite Unterdrückung als kurz aufblitzendes Konsolenfenster anzeigt,
  bei jedem Polling-Intervall. Fix in `system_monitor.rs` via
  `CREATE_NO_WINDOW`-Flag auf Windows.
- **Quelle `bild.de` funktionierte nicht**: der Homepage-Link selbst ist
  kein RSS-Feed. `source_manager.rs` sucht jetzt automatisch nach dem
  `<link rel="alternate" type="application/rss+xml">`-Autodiscovery-Tag,
  das praktisch jede Nachrichtenseite einbettet, und speichert die dabei
  gefundene tatsächliche Feed-URL statt der eingegebenen Homepage-URL.

## UI-Lesbarkeit & echte Medienintegration (dritte Runde)

- **Dropdowns kaum lesbar behoben**: WebKit/GTK rendert `<select>` ohne
  `appearance: none` mit systemeigenem (hellem) Chrome, das unser
  Dark-Theme nur teilweise überschreiben konnte. Jetzt mit `appearance:
  none` + eigenem SVG-Pfeil in `SettingsPanel.svelte` und
  `AddWidgetMenu.svelte` behoben. Platzhaltertext ist jetzt global über
  `::placeholder` in `app.css` eingefärbt statt dem (auf Dark-Hintergrund
  oft zu blassen) Browser-Standard.
- **Quelle-hinzufügen-Feld deutlicher**: klareres Label
  ("Link zur Webseite oder zum RSS-Feed"), konkreterer Platzhalter,
  Auto-Fokus beim Öffnen.
- **Medienwiedergabe ist jetzt echt**, nicht mehr die Attrappe aus dem
  ersten Entwurf: `core/media.rs` liest über **MPRIS** (D-Bus) aus, was
  gerade läuft — das erfasst automatisch auch Chromium-Tabs (Brave, Chrome)
  mit aktiver Media-Session-API, also z. B. Deezer im Browser, ohne
  seitenspezifischen Code. Play/Pause/Weiter/Zurück steuern den jeweils
  aktiven Player. Nur Linux implementiert (passend zu CachyOS); Windows
  bräuchte separat die SMTC-API, ist als TODO vermerkt.

## Bekannte Grenze: Systemübersicht vs. natives Linux-Tool

Die CPU-/Systemlast-Werte werden nie exakt mit KDE System Monitor
übereinstimmen — beide berechnen aus denselben Kernel-Werten, aber mit
unterschiedlichen Sampling-Fenstern und Glättung. Die letzten Fixes
(persistente Messung, sauberer Startwert) haben die größten Ausreißer
behoben; für Werte, die bis auf die Nachkommastelle übereinstimmen, müsste
man exakt Plasmas Berechnungsmethode nachbauen, was den Aufwand kaum wert
ist. Falls die Abweichung nach dem nächsten Test noch groß ist (nicht nur
1-2 Prozentpunkte), sag Bescheid mit einem Vergleich beider Werte
gleichzeitig — dann schauen wir gezielt weiter.

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
