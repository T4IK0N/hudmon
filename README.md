# Hudmon

**Minimalist retro overlay with live FPS, CPU, GPU and RAM — Windows & Linux**  
**Minimalistyczny, retro overlay z FPS, CPU, GPU i RAM na żywo — Windows i Linux**

[English](#english) · [Polski](#polski)

---

# English

## What is it?

Hudmon draws a tiny, always-on-top, click-through overlay in a corner of your screen showing live **FPS**, **CPU** load, **GPU** load, **RAM** usage and **CPU/GPU temperatures**. It is written in Rust, rendered in software with an 8x8 bitmap font (old-school look), does not touch your GPU and ships as a few-MB binary.

It consists of two programs that live in the same folder:

| File | Role |
|---|---|
| `hudmon` / `hudmon.exe` | **Settings window** (English / Polish). Starts and closes the overlay and changes its look live. |
| `hudmon-overlay` / `hudmon-overlay.exe` | **The overlay itself.** No window frame, no console, not shown in the taskbar (Windows). |

### Features

- Show/hide FPS, CPU, GPU, RAM and temperatures independently (e.g. only FPS).
- Optional labels: `144` instead of `FPS 144`.
- Font size 1x–6x, background color (8 presets or any hex color), background opacity 0–100 %, optional border, any screen corner.
- Changes apply live (the overlay reloads its config about every 0.5 s).
- Only one overlay runs at a time (starting a new one closes the old one).
- Settings UI in English and Polish.

---

## Quick start — Windows (prebuilt)

1. Download the latest Windows archive from the **Releases** page and extract it anywhere. Keep `hudmon.exe` and `hudmon-overlay.exe` in the same folder.
2. **For FPS:** download the *console* **PresentMon** (`PresentMon-x.y.z-x64.exe`) from <https://github.com/GameTechDev/PresentMon/releases> and put it in the same folder. Any file name starting with `PresentMon` is detected automatically. (It is not bundled with hudmon — download it from its official page.)
3. **For CPU temperature:** install and run **LibreHardwareMonitor** (<https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>) as administrator and enable *Options → Remote Web Server → Run* (port 8085). See [CPU temperature on Windows](#cpu-temperature-on-windows).
4. Run **`hudmon.exe` as administrator** (right-click → *Run as administrator*) and press **Start**. The overlay inherits the rights of the settings window. Administrator rights are needed for FPS (PresentMon uses Event Tracing for Windows).
5. Windows SmartScreen may warn about the unsigned executables: *More info → Run anyway*.

> **Games:** use *windowed* or *borderless* mode. An overlay window cannot be drawn over exclusive fullscreen.

---

## Build from source

Requires **Rust 1.85 or newer**.

### Windows

1. Install Rust from <https://rustup.rs>. When asked, install the **Visual Studio Build Tools** with the *Desktop development with C++* workload (MSVC + Windows SDK).
2. Build:
   ```bat
   git clone https://github.com/<USER>/<REPO>.git
   cd <REPO>
   cargo build --release
   ```
3. The programs are in `target\release\`: `hudmon.exe` and `hudmon-overlay.exe`.

### Linux — Manjaro / Arch

```bash
sudo pacman -S --needed base-devel git rust
git clone https://github.com/<USER>/<REPO>.git
cd <REPO>
cargo build --release
```
### Linux — other distributions

Install a C compiler/linker, `git` and **rustup** (distribution packages of Rust are often too old), then build as above:

```bash
# Debian / Ubuntu
sudo apt install build-essential git curl
# Fedora
sudo dnf install gcc git curl
# then, on any of them:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Running on Linux (Manjaro)

After building, the two programs are in `target/release/`. Keep them in the same directory (the **Start** button launches `hudmon-overlay` from the folder where `hudmon` lives).

```bash
./target/release/hudmon            # settings window — press "Start" to launch the overlay
# or start the overlay directly:
./target/release/hudmon-overlay
```

Optional system-wide install:

```bash
sudo install -Dm755 target/release/hudmon         /usr/local/bin/hudmon
sudo install -Dm755 target/release/hudmon-overlay /usr/local/bin/hudmon-overlay
```

### Wayland vs X11

Manjaro KDE Plasma and GNOME use **Wayland** by default. On Wayland a normal window cannot position itself or stay on top of other windows, and this build has no per-pixel transparency there. Run hudmon through **XWayland** instead:

```bash
env -u WAYLAND_DISPLAY hudmon
```

The overlay started from the settings window inherits this. On an X11 session nothing special is needed. Background transparency requires a running compositor.

### GPU and temperature requirements

- **NVIDIA:** proprietary driver (the `nvidia-utils` package provides `libnvidia-ml`).
- **AMD:** the `amdgpu` kernel driver (usage is read from `/sys/class/drm/card*/device/gpu_busy_percent`).
- **CPU temperature:** hwmon sensors (`coretemp` for Intel, `k10temp` for AMD) — loaded by default.

---

## Settings

Run `hudmon` / `hudmon.exe`. Every click is saved immediately and the running overlay picks it up within about half a second. The language of the settings window is switched with the **PL / EN** buttons in the top-right corner.

You can also edit the config file by hand:

| OS | Path |
|---|---|
| Windows | `%APPDATA%\hudmon\hudmon.ini` |
| Linux | `~/.config/hudmon/hudmon.ini` (respects `XDG_CONFIG_HOME`) |

| Key | Values | Meaning |
|---|---|---|
| `fps`, `cpu`, `gpu`, `ram` | `0` / `1` | Show the row |
| `temps` | `0` / `1` | Show CPU/GPU temperatures |
| `labels` | `0` / `1` | Show `FPS`, `CPU`… before the values |
| `border` | `0` / `1` | Draw a border |
| `font` | `1`–`6` | Font size: 1x = 16 px glyphs, 2x = 24 px, 3x = 32 px … 6x = 56 px |
| `bg_color` | hex `RRGGBB` | Background color |
| `bg_opacity` | `0`–`100` | Background opacity in % (text is always opaque) |
| `corner` | `0`–`3` | 0 top-left, 1 top-right, 2 bottom-left, 3 bottom-right |
| `lang` | `pl` / `en` | Language of the settings window |

Temperatures are green below 70 °C, amber from 70 °C and red from 85 °C.

---

## Where the data comes from

| Metric | Windows | Linux |
|---|---|---|
| CPU load, RAM | OS counters (via `sysinfo`) | `/proc` (via `sysinfo`) |
| GPU load | NVIDIA via NVML. AMD/Intel: not supported yet (`n/a`) | NVIDIA via NVML, AMD via `amdgpu` sysfs. Intel: not supported yet (`n/a`) |
| CPU temperature | LibreHardwareMonitor (web server) | hwmon sensors |
| GPU temperature | NVIDIA via NVML; others via LibreHardwareMonitor | NVIDIA via NVML, AMD via hwmon |
| FPS | **PresentMon** (Event Tracing for Windows) | *Experimental, not working live* — see below |

### FPS on Windows

Hudmon starts PresentMon in the background and counts frames per second of the application that presented the most frames during the last second (`dwm.exe` is ignored). PresentMon does **not** inject anything into the game, so it works with most anti-cheat systems — but there is no guarantee, use it at your own risk.

If you use PresentMon 1.x, whose flags use a single dash:
```bat
hudmon-overlay.exe --presentmon-args "-output_stdout -stop_existing_session"
```

### CPU temperature on Windows

Windows does not expose the CPU temperature to normal programs, so hudmon reads it from **LibreHardwareMonitor**, which has its own sensor driver:

1. Run LibreHardwareMonitor as administrator.
2. *Options → Remote Web Server → **Run*** (default port 8085; use `--lhm-port N` for another port).
3. Check `http://127.0.0.1:8085` in your browser — you should see a page with sensors. hudmon then shows the temperature within about 2 seconds.

---

## Command-line options (overlay)

Normally you do not need them.

| Option | Meaning |
|---|---|
| `--interval MS` | Refresh interval (default 500, minimum 200) |
| `--presentmon PATH` | Path to PresentMon (Windows). Default: auto-detect next to the executable |
| `--presentmon-args "…"` | Arguments for PresentMon (default `--output_stdout --stop_existing_session`) |
| `--lhm-port N` | LibreHardwareMonitor web server port (default 8085) |
| `--mangohud-dir DIR` | *Experimental* (Linux): folder with MangoHud CSV logs |

---

## Troubleshooting

Diagnostics are written to **`hudmon.log`** next to the executables (if the folder is writable). PresentMon's own errors go to `%TEMP%\hudmon-presentmon-stderr.log`.

| Problem | What to do |
|---|---|
| FPS shows `n/a` (Windows) | Run as administrator (or join *Performance Log Users*). Make sure a `PresentMon*.exe` is next to `hudmon-overlay.exe`. Check `hudmon.log`. FPS appears only while a game or app is presenting frames. |
| CPU temperature shows `n/a` (Windows) | LibreHardwareMonitor must be running with the Remote Web Server enabled. Look for `LHM:` lines in `hudmon.log`. |
| Two overlays / overlapping numbers | Close all `hudmon-overlay.exe` in Task Manager once. Newer builds close old copies automatically — unless the old one runs as administrator and the new one does not. |
| Overlay not visible in a game | Use windowed/borderless mode; exclusive fullscreen hides overlay windows. |
| Overlay does not resize after changing the font | Look for `UpdateLayeredWindow` lines in `hudmon.log` and open an issue with them. |
| GPU shows `n/a` | NVIDIA works everywhere. AMD works on Linux only; Intel is not supported yet. |

---

## Project layout

```
src/
  main.rs               entry point of hudmon-overlay
  bin/hudmon.rs         entry point of hudmon (settings)
  lib.rs                shared library
  config.rs             hudmon.ini load/save
  render.rs             software renderer (8x8 font, premultiplied ARGB)
  overlay.rs            overlay window and main loop
  settings.rs           settings window (PL/EN)
  instance.rs           single instance, starting/closing the overlay
  present_win.rs        Windows: per-pixel transparent window (UpdateLayeredWindow)
  metrics/
    mod.rs              Provider trait + Snapshot (the UI knows nothing about the OS)
    system.rs           CPU / RAM / CPU temperature
    gpu.rs              NVML, amdgpu sysfs
    fps.rs              PresentMon (Windows), MangoHud log (Linux, experimental)
    lhm.rs              temperatures from LibreHardwareMonitor
assets/icon.ico         optional icon (not in the repository by default)
```

New data sources are added by implementing the `Provider` trait in `src/metrics/`.

---

## Limitations and roadmap

- **Linux FPS does not work live yet.** Linux has no universal way to measure the FPS of another process, so FPS has to come from inside the game (a Vulkan layer). The current `--mangohud-dir` reader tails a MangoHud CSV log, but MangoHud keeps its log in memory and appears to write the file only when logging stops, so the value will not update live. A proper solution (own Vulkan layer) is not implemented yet.
- GPU load: Windows with AMD/Intel and Linux with Intel are not supported.
- Wayland: no always-on-top, no window positioning and no transparency — use XWayland (see above).
- No global hotkeys and no tray icon yet.
- The executables are not code-signed.
- Support for other screens

---

# Polski

## Co to jest?

Hudmon rysuje mały overlay w rogu ekranu (zawsze na wierzchu, „przezroczysty” dla myszy), który na żywo pokazuje **FPS**, obciążenie **CPU** i **GPU**, zużycie **RAM** oraz **temperatury CPU/GPU**. Jest napisany w Ruście, rysowany programowo czcionką bitmapową 8x8 (w starym stylu), nie obciąża karty graficznej i waży kilka MB.

Składa się z dwóch programów leżących w jednym folderze:

| Plik | Rola |
|---|---|
| `hudmon` / `hudmon.exe` | **Okno ustawień** (polski / angielski). Uruchamia i zamyka overlay oraz zmienia jego wygląd na żywo. |
| `hudmon-overlay` / `hudmon-overlay.exe` | **Sam overlay.** Bez ramki okna, bez konsoli, nie widać go na pasku zadań (Windows). |

### Funkcje

- Osobne włączanie/wyłączanie FPS, CPU, GPU, RAM i temperatur (np. same FPS).
- Opcjonalne napisy: `144` zamiast `FPS 144`.
- Czcionka 1x–6x, kolor tła (8 gotowych lub dowolny hex), krycie tła 0–100 %, opcjonalna ramka, dowolny róg ekranu.
- Zmiany działają na żywo (overlay przeładowuje ustawienia mniej więcej co 0,5 s).
- Działa tylko jeden overlay naraz (uruchomienie nowego zamyka stary).
- Okno ustawień po polsku i angielsku.

### Stan projektu

| Platforma | Stan |
|---|---|
| **Windows 11** | Tworzony i testowany. |
| **Linux** | Buduje się i przechodzą testy jednostkowe, ale **nie został jeszcze sprawdzony na prawdziwym pulpicie** (zwłaszcza Wayland). **FPS na żywo na Linuksie jeszcze nie działa** — zobacz [Ograniczenia](#ograniczenia-i-plany). CPU/GPU/RAM/temperatury są zaimplementowane. |

---

## Szybki start — Windows (gotowe pliki)

1. Pobierz najnowsze archiwum dla Windowsa ze strony **Releases** i rozpakuj w dowolnym miejscu. `hudmon.exe` i `hudmon-overlay.exe` muszą leżeć w tym samym folderze.
2. **Do FPS:** pobierz *konsolowy* **PresentMon** (`PresentMon-x.y.z-x64.exe`) z <https://github.com/GameTechDev/PresentMon/releases> i wrzuć go do tego samego folderu. Wykrywany jest każdy plik, którego nazwa zaczyna się od `PresentMon`. (Nie jest dołączony do paczki — pobierz go ze strony oficjalnej.)
3. **Do temperatury CPU:** zainstaluj i uruchom jako administrator **LibreHardwareMonitor** (<https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>) i włącz *Options → Remote Web Server → Run* (port 8085). Zobacz [Temperatura CPU na Windowsie](#temperatura-cpu-na-windowsie).
4. Uruchom **`hudmon.exe` jako administrator** (prawy przycisk → *Uruchom jako administrator*) i kliknij **Uruchom**. Overlay dziedziczy uprawnienia okna ustawień. Uprawnienia administratora są potrzebne do FPS (PresentMon korzysta z Event Tracing for Windows).
5. Windows SmartScreen może ostrzegać przed niepodpisanymi plikami: *Więcej informacji → Uruchom mimo to*.

> **Gry:** używaj trybu *okno* lub *okno bez ramek*. Overlay nie zostanie wyświetlony nad „prawdziwym” pełnym ekranem.

---

## Budowanie ze źródeł

Wymagany jest **Rust 1.85 lub nowszy**.

### Windows

1. Zainstaluj Rusta z <https://rustup.rs>. Gdy instalator zapyta, zainstaluj **Visual Studio Build Tools** z pakietem *Programowanie aplikacji klasycznych w C++* (MSVC + Windows SDK).
2. Zbuduj:
   ```bat
   git clone https://github.com/<USER>/<REPO>.git
   cd <REPO>
   cargo build --release
   ```
3. Programy znajdziesz w `target\release\`: `hudmon.exe` i `hudmon-overlay.exe`.

### Linux — Manjaro / Arch

```bash
sudo pacman -S --needed base-devel git rust
git clone https://github.com/<USER>/<REPO>.git
cd <REPO>
cargo build --release
```

### Linux — inne dystrybucje

Zainstaluj kompilator C/linker, `git` i **rustup** (pakiety Rusta w dystrybucjach bywają za stare), a potem zbuduj jak wyżej:

```bash
# Debian / Ubuntu
sudo apt install build-essential git curl
# Fedora
sudo dnf install gcc git curl
# potem, na każdej z nich:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Uruchamianie na Linuksie (Manjaro)

Po zbudowaniu oba programy leżą w `target/release/`. Trzymaj je w jednym katalogu (przycisk **Uruchom** startuje `hudmon-overlay` z folderu, w którym leży `hudmon`).

```bash
./target/release/hudmon            # okno ustawień — kliknij „Uruchom”, żeby włączyć overlay
# albo uruchom sam overlay:
./target/release/hudmon-overlay
```

Opcjonalna instalacja w systemie:

```bash
sudo install -Dm755 target/release/hudmon         /usr/local/bin/hudmon
sudo install -Dm755 target/release/hudmon-overlay /usr/local/bin/hudmon-overlay
```

### Wayland a X11

Manjaro KDE Plasma i GNOME domyślnie używają **Waylanda**. Na Waylandzie zwykłe okno nie może samo ustawić pozycji ani zostać na wierzchu, a w tej wersji nie ma tam też przezroczystości per piksel. Uruchom hudmon przez **XWayland**:

```bash
env -u WAYLAND_DISPLAY hudmon
```

Overlay uruchomiony z okna ustawień odziedziczy to ustawienie. W sesji X11 nie trzeba nic robić. Przezroczystość tła wymaga działającego kompozytora.

### Wymagania dla GPU i temperatur

- **NVIDIA:** sterownik własnościowy (pakiet `nvidia-utils` dostarcza `libnvidia-ml`).
- **AMD:** sterownik jądra `amdgpu` (użycie jest czytane z `/sys/class/drm/card*/device/gpu_busy_percent`).
- **Temperatura CPU:** czujniki hwmon (`coretemp` dla Intela, `k10temp` dla AMD) — domyślnie załadowane.

---

## Ustawienia

Uruchom `hudmon` / `hudmon.exe`. Każde kliknięcie zapisuje się od razu, a działający overlay przejmuje zmianę w ciągu około pół sekundy. Język okna ustawień zmieniasz przyciskami **PL / EN** w prawym górnym rogu. Napisy w oknie ustawień nie mają polskich znaków diakrytycznych (czcionka bitmapowa 8x8 ich nie zawiera).

Plik konfiguracyjny możesz też edytować ręcznie:

| System | Ścieżka |
|---|---|
| Windows | `%APPDATA%\hudmon\hudmon.ini` |
| Linux | `~/.config/hudmon/hudmon.ini` (uwzględnia `XDG_CONFIG_HOME`) |

| Klucz | Wartości | Znaczenie |
|---|---|---|
| `fps`, `cpu`, `gpu`, `ram` | `0` / `1` | Pokaż wiersz |
| `temps` | `0` / `1` | Pokaż temperatury CPU/GPU |
| `labels` | `0` / `1` | Pokaż napisy `FPS`, `CPU`… przed wartościami |
| `border` | `0` / `1` | Rysuj ramkę |
| `font` | `1`–`6` | Rozmiar czcionki: 1x = litery 16 px, 2x = 24 px, 3x = 32 px … 6x = 56 px |
| `bg_color` | hex `RRGGBB` | Kolor tła |
| `bg_opacity` | `0`–`100` | Krycie tła w % (tekst jest zawsze nieprzezroczysty) |
| `corner` | `0`–`3` | 0 lewy górny, 1 prawy górny, 2 lewy dolny, 3 prawy dolny |
| `lang` | `pl` / `en` | Język okna ustawień |

Temperatury są zielone poniżej 70 °C, bursztynowe od 70 °C i czerwone od 85 °C.

---

## Skąd pochodzą dane

| Wartość | Windows | Linux |
|---|---|---|
| Obciążenie CPU, RAM | liczniki systemu (przez `sysinfo`) | `/proc` (przez `sysinfo`) |
| Obciążenie GPU | NVIDIA przez NVML. AMD/Intel: jeszcze niewspierane (`n/a`) | NVIDIA przez NVML, AMD przez sysfs `amdgpu`. Intel: jeszcze niewspierany (`n/a`) |
| Temperatura CPU | LibreHardwareMonitor (serwer WWW) | czujniki hwmon |
| Temperatura GPU | NVIDIA przez NVML; inne przez LibreHardwareMonitor | NVIDIA przez NVML, AMD przez hwmon |
| FPS | **PresentMon** (Event Tracing for Windows) | *Eksperymentalne, nie działa na żywo* — zobacz niżej |

### FPS na Windowsie

Hudmon uruchamia w tle PresentMon i liczy klatki na sekundę aplikacji, która w ostatniej sekundzie wyświetliła ich najwięcej (`dwm.exe` jest pomijany). PresentMon **niczego nie wstrzykuje** do gry, więc zwykle działa z systemami anti-cheat — ale nie ma gwarancji, używasz na własną odpowiedzialność.

Jeśli używasz PresentMon 1.x, który przyjmuje flagi z jednym myślnikiem:
```bat
hudmon-overlay.exe --presentmon-args "-output_stdout -stop_existing_session"
```

### Temperatura CPU na Windowsie

Windows nie udostępnia temperatury CPU zwykłym programom, dlatego hudmon czyta ją z **LibreHardwareMonitor**, który ma własny sterownik czujników:

1. Uruchom LibreHardwareMonitor jako administrator.
2. *Options → Remote Web Server → **Run*** (domyślny port 8085; inny port podasz przez `--lhm-port N`).
3. Sprawdź w przeglądarce `http://127.0.0.1:8085` — powinna się pokazać strona z czujnikami. Po chwili (do ok. 2 s) hudmon pokaże temperaturę.

---

## Opcje wiersza poleceń (overlay)

Zwykle nie są potrzebne.

| Opcja | Znaczenie |
|---|---|
| `--interval MS` | Odświeżanie (domyślnie 500, minimum 200) |
| `--presentmon ŚCIEŻKA` | Ścieżka do PresentMon (Windows). Domyślnie szukany obok pliku wykonywalnego |
| `--presentmon-args "…"` | Argumenty PresentMon (domyślnie `--output_stdout --stop_existing_session`) |
| `--lhm-port N` | Port serwera WWW LibreHardwareMonitor (domyślnie 8085) |
| `--mangohud-dir KATALOG` | *Eksperymentalne* (Linux): katalog z logami CSV MangoHuda |

---

## Rozwiązywanie problemów

Diagnostyka trafia do **`hudmon.log`** obok plików wykonywalnych (jeśli folder jest zapisywalny). Błędy samego PresentMon są w `%TEMP%\hudmon-presentmon-stderr.log`.

| Problem | Co zrobić |
|---|---|
| FPS pokazuje `n/a` (Windows) | Uruchom jako administrator (albo dołącz do grupy *Performance Log Users*). Sprawdź, czy obok `hudmon-overlay.exe` leży `PresentMon*.exe`. Zajrzyj do `hudmon.log`. FPS pojawia się tylko, gdy gra lub aplikacja faktycznie wyświetla klatki. |
| Temperatura CPU `n/a` (Windows) | LibreHardwareMonitor musi działać z włączonym Remote Web Server. Szukaj linii `LHM:` w `hudmon.log`. |
| Dwa overlaye / nakładające się liczby | Raz zamknij wszystkie `hudmon-overlay.exe` w Menedżerze zadań. Nowe wersje same zamykają stare kopie — chyba że stara działa jako administrator, a nowa nie. |
| Overlay niewidoczny w grze | Użyj trybu okno/okno bez ramek; pełny ekran na wyłączność ukrywa okna overlay. |
| Overlay nie powiększa się po zmianie czcionki | Poszukaj w `hudmon.log` linii `UpdateLayeredWindow` i zgłoś issue z nimi. |
| GPU pokazuje `n/a` | NVIDIA działa wszędzie. AMD tylko na Linuksie; Intel jeszcze nie jest wspierany. |

---

## Struktura projektu

```
src/
  main.rs               punkt wejścia hudmon-overlay
  bin/hudmon.rs         punkt wejścia hudmon (ustawienia)
  lib.rs                wspólna biblioteka
  config.rs             wczytywanie/zapis hudmon.ini
  render.rs             renderer programowy (czcionka 8x8, premultiplied ARGB)
  overlay.rs            okno overlaya i główna pętla
  settings.rs           okno ustawień (PL/EN)
  instance.rs           jedna kopia, uruchamianie/zamykanie overlaya
  present_win.rs        Windows: okno z przezroczystością per piksel (UpdateLayeredWindow)
  metrics/
    mod.rs              trait Provider + Snapshot (UI nic nie wie o systemie)
    system.rs           CPU / RAM / temperatura CPU
    gpu.rs              NVML, sysfs amdgpu
    fps.rs              PresentMon (Windows), log MangoHuda (Linux, eksperymentalne)
    lhm.rs              temperatury z LibreHardwareMonitor
assets/icon.ico         opcjonalna ikona (domyślnie nie ma jej w repozytorium)
```

Nowe źródło danych dodajesz, implementując trait `Provider` w `src/metrics/`.

---

## Ograniczenia i plany

- **FPS na Linuksie nie działa jeszcze na żywo.** Linux nie ma uniwersalnego sposobu mierzenia FPS cudzego procesu, więc FPS musi pochodzić z wnętrza gry (warstwa Vulkana). Obecny odczyt `--mangohud-dir` czyta koniec logu CSV MangoHuda, ale MangoHud trzyma log w pamięci i wygląda na to, że zapisuje plik dopiero po zakończeniu logowania, więc wartość nie będzie się odświeżać. Właściwe rozwiązanie (własna warstwa Vulkana) nie jest jeszcze zaimplementowane.
- Obciążenie GPU: Windows z AMD/Intel oraz Linux z Intelem nie są wspierane.
- Wayland: brak „zawsze na wierzchu”, brak ustawiania pozycji okna i brak przezroczystości — użyj XWayland (patrz wyżej).
- Brak globalnych skrótów klawiszowych i ikony w zasobniku.
- Pliki wykonywalne nie są podpisane cyfrowo.
- Obsługa innych ekranów
