# Dockermon user guide / Instrukcja obsługi Dockermon

[English](#english) · [Polski](#polski)

## English

### Requirements

- A Linux desktop. Dockermon is designed and tested primarily for Zorin OS 18.
- A running local Docker Engine.
- Docker Compose available as the `docker compose` command.
- Permission for the current user to access the Docker socket.

To grant Docker access on a typical Debian or Ubuntu-based system, run:

```bash
sudo usermod -aG docker "$USER"
```

Log out and back in after changing group membership. Membership in the `docker` group grants
root-equivalent access to the machine. Verify the setup before starting Dockermon:

```bash
docker info
docker compose version
```

### Install or update

Install Dockermon from its signed APT repository to receive future updates:

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSLo /tmp/dockermon-archive-keyring.gpg \
  https://apt.forseti.pl/dockermon-archive-keyring.gpg
sudo install -m 0644 /tmp/dockermon-archive-keyring.gpg \
  /etc/apt/keyrings/dockermon-archive-keyring.gpg
curl -fsSLo /tmp/dockermon.sources https://apt.forseti.pl/dockermon.sources
sudo install -m 0644 /tmp/dockermon.sources \
  /etc/apt/sources.list.d/dockermon.sources
sudo apt update
sudo apt install dockermon
```

Alternatively, install a downloaded Debian package directly:

```bash
sudo apt install ./Dockermon_0.1.3_amd64.deb
```

The same command updates an existing installation when the package file contains a newer version.
Launch Dockermon from the application menu. Closing the main window keeps Dockermon running in the
system tray; use **Quit** in the tray menu to exit completely.

### Add and manage projects

Dockermon automatically discovers running local containers and Docker Compose metadata. A Compose
project can also be registered explicitly from the **Projects** view:

- **Add directory** selects a directory containing a Compose configuration.
- **Add Compose files** selects one or more Compose files belonging to the same project.

Registered projects remain available after `docker compose down`, so their services can be started
again from Dockermon. Use the profile controls next to a project name to enable the Compose profiles
needed for that project.

The **Active** switch marks the projects that currently matter. Active projects determine the
aggregate status shown by the tray icon and which projects appear in the tray menu. If no project is
active, the tray menu shows all registered projects.

Removing a project from Dockermon removes only its saved registration. It does not remove containers,
volumes, images, networks, or Compose files.

### Control services and inspect logs

Each service row shows its status, CPU usage, memory usage, and published ports. The buttons at the
right of a row start or resume, stop, restart, and open the logs for that service.

Use the checkboxes to select services within one project, then choose **Start selected** or
**Stop selected**. These bulk actions affect only the selected services in that project. The
**Containers** view lists standalone containers that are not part of a Compose project.

The log drawer follows new output by default. Disable **Follow** to inspect earlier lines and use
**Clear** to clear only the visible log buffer; it does not delete Docker logs.

### Use the tray menu

The tray icon summarizes Docker connectivity and the state of active projects. Its project submenus
provide quick service actions without opening the main window.

> **Warning:** **Stop all** stops every running or paused container visible to the local Docker
> Engine, including containers outside registered or active projects.

### Preferences and saved data

The **Preferences** view controls the interface language, light or dark appearance, and automatic
startup after login. Automatic startup launches Dockermon hidden in the tray.

Dockermon stores its configuration in `$XDG_CONFIG_HOME/dockermon/config.json` or, when
`XDG_CONFIG_HOME` is not set, in `~/.config/dockermon/config.json`.

### Troubleshooting

- **Docker unavailable:** start Docker Engine, then verify it with `docker info`.
- **Docker access denied:** add the user to the `docker` group, log out and back in, then run
  `docker info` again.
- **Docker Compose unavailable:** install the Docker Compose plugin and verify
  `docker compose version`.
- **A project cannot be started:** confirm that its saved Compose files still exist and that its
  required environment files and variables are available.
- **The tray icon is missing:** ensure that the desktop's AppIndicator or StatusNotifier support is
  enabled. Zorin OS provides this integration by default.

To uninstall Dockermon while preserving its user configuration, run:

```bash
sudo apt remove dockermon
```

## Polski

### Wymagania

- Środowisko graficzne Linux. Dockermon jest projektowany i testowany przede wszystkim w Zorin OS 18.
- Uruchomiony lokalny Docker Engine.
- Docker Compose dostępny jako polecenie `docker compose`.
- Uprawnienia bieżącego użytkownika do gniazda Dockera.

Aby przyznać dostęp do Dockera w typowym systemie opartym na Debianie lub Ubuntu, uruchom:

```bash
sudo usermod -aG docker "$USER"
```

Po zmianie przynależności do grupy wyloguj się i zaloguj ponownie. Członkostwo w grupie `docker`
daje uprawnienia równoważne uprawnieniom administratora systemu. Przed uruchomieniem Dockermon
sprawdź konfigurację:

```bash
docker info
docker compose version
```

### Instalacja i aktualizacja

Zainstaluj Dockermon z podpisanego repozytorium APT, aby otrzymywać przyszłe aktualizacje:

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSLo /tmp/dockermon-archive-keyring.gpg \
  https://apt.forseti.pl/dockermon-archive-keyring.gpg
sudo install -m 0644 /tmp/dockermon-archive-keyring.gpg \
  /etc/apt/keyrings/dockermon-archive-keyring.gpg
curl -fsSLo /tmp/dockermon.sources https://apt.forseti.pl/dockermon.sources
sudo install -m 0644 /tmp/dockermon.sources \
  /etc/apt/sources.list.d/dockermon.sources
sudo apt update
sudo apt install dockermon
```

Możesz też bezpośrednio zainstalować pobrany pakiet Debiana:

```bash
sudo apt install ./Dockermon_0.1.3_amd64.deb
```

To samo polecenie zaktualizuje istniejącą instalację, jeśli plik zawiera nowszą wersję pakietu.
Uruchom Dockermon z menu aplikacji. Zamknięcie głównego okna pozostawia Dockermon uruchomiony w
zasobniku systemowym; aby całkowicie zakończyć aplikację, wybierz **Zakończ** w menu zasobnika.

### Dodawanie projektów i zarządzanie nimi

Dockermon automatycznie wykrywa działające lokalne kontenery oraz metadane Docker Compose. Projekt
Compose można też zarejestrować ręcznie w widoku **Projekty**:

- **Dodaj katalog** wybiera katalog zawierający konfigurację Compose.
- **Dodaj pliki Compose** wybiera jeden lub więcej plików Compose należących do tego samego projektu.

Zarejestrowane projekty pozostają dostępne po wykonaniu `docker compose down`, dlatego ich serwisy
można ponownie uruchomić z Dockermon. Przełączniki profili obok nazwy projektu służą do włączania
potrzebnych profili Compose.

Przełącznik **Aktywny** oznacza projekty, które są obecnie istotne. Aktywne projekty wpływają na
zbiorczy stan ikony zasobnika oraz na listę projektów w jego menu. Jeśli żaden projekt nie jest
aktywny, menu zasobnika pokazuje wszystkie zarejestrowane projekty.

Usunięcie projektu z Dockermon usuwa wyłącznie jego zapisaną rejestrację. Nie usuwa kontenerów,
wolumenów, obrazów, sieci ani plików Compose.

### Sterowanie serwisami i przeglądanie logów

Wiersz serwisu pokazuje jego stan, użycie procesora i pamięci oraz opublikowane porty. Przyciski po
prawej stronie uruchamiają lub wznawiają, zatrzymują i restartują serwis oraz otwierają jego logi.

Zaznacz serwisy w obrębie jednego projektu, a następnie wybierz **Uruchom zaznaczone** albo
**Zatrzymaj zaznaczone**. Te operacje zbiorcze dotyczą wyłącznie zaznaczonych serwisów w danym
projekcie. Widok **Kontenery** pokazuje samodzielne kontenery, które nie należą do projektu Compose.

Szuflada logów domyślnie śledzi nowe wpisy. Wyłącz **Śledzenie**, aby przejrzeć wcześniejsze wiersze.
Przycisk **Wyczyść** usuwa tylko zawartość widoczną w aplikacji — nie kasuje logów Dockera.

### Menu zasobnika

Ikona zasobnika podsumowuje połączenie z Dockerem i stan aktywnych projektów. Podmenu projektów
pozwalają szybko sterować serwisami bez otwierania głównego okna.

> **Uwaga:** **Zatrzymaj wszystko** zatrzymuje każdy działający lub wstrzymany kontener widoczny dla
> lokalnego Docker Engine, również kontenery spoza zarejestrowanych i aktywnych projektów.

### Preferencje i zapisane dane

Widok **Preferencje** pozwala wybrać język interfejsu, jasny lub ciemny wygląd oraz automatyczne
uruchamianie po zalogowaniu. Autostart uruchamia Dockermon w tle, ukryty w zasobniku.

Dockermon zapisuje konfigurację w `$XDG_CONFIG_HOME/dockermon/config.json`, a jeśli zmienna
`XDG_CONFIG_HOME` nie jest ustawiona — w `~/.config/dockermon/config.json`.

### Rozwiązywanie problemów

- **Docker jest niedostępny:** uruchom Docker Engine, a następnie sprawdź go poleceniem `docker info`.
- **Brak dostępu do Dockera:** dodaj użytkownika do grupy `docker`, wyloguj się i zaloguj ponownie,
  a następnie jeszcze raz uruchom `docker info`.
- **Docker Compose jest niedostępny:** zainstaluj wtyczkę Docker Compose i sprawdź polecenie
  `docker compose version`.
- **Nie można uruchomić projektu:** sprawdź, czy zapisane pliki Compose nadal istnieją oraz czy są
  dostępne wymagane pliki środowiskowe i zmienne.
- **Brakuje ikony zasobnika:** upewnij się, że obsługa AppIndicator lub StatusNotifier jest włączona.
  Zorin OS domyślnie zapewnia tę integrację.

Aby odinstalować Dockermon bez usuwania konfiguracji użytkownika, uruchom:

```bash
sudo apt remove dockermon
```
