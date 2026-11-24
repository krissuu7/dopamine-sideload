# dopamine-sideload

Jedno repozytorium zawiera monitor USB, sideloader, ekran ustawień i updater IPA.
Oryginalna logika sideloadingu pozostaje oparta na `isideload`; ustawienia nie
przechowują haseł w plikach. Poświadczenia zapisuje biblioteka Rust przez
standardowy interfejs Secret Service; aplikacja nie wywołuje narzędzi ani API
zależnych od konkretnego środowiska graficznego. System musi udostępniać zgodny
menedżer haseł. Kod 2FA nadal jest pytany przez okno GTK dopiero przy logowaniu
Apple.

## Instalacja ze źródeł

Instalator rozpoznaje DNF, APT, Pacman i Zypper; zapyta przed doinstalowaniem
pakietów systemowych. Buduje programy lokalnie, a gdy systemowy Rust jest starszy
niż 1.88, może zainstalować toolchain stable przez rustup. Wymaga systemd z sesją
użytkownika, `usbmuxd`, GTK4, libadwaita i usługi menedżera haseł zgodnej z
Secret Service. Dystrybucje bez systemd user session nie są obecnie obsługiwane.

Po opublikowaniu repozytorium ustaw jego nazwę w poleceniu:

```sh
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

Jednolinijkowiec pobiera bootstrap z repozytorium i instaluje bieżącą gałąź
`main`. Można wskazać inną gałąź przez `DOPAMINE_REF`.

## Zachowanie

- Demon wykrywa urządzenia USB przez `usbmuxd`, bez zapisanego UDID telefonu.
- Przy kilku telefonach uruchamia sideload dla pierwszego wykrytego urządzenia;
  dokładny UDID przekazuje do procesu sideloadera.
- Aplikacja ustawień pozwala włączyć/wyłączyć usługę i zapisać Apple ID oraz
  hasło w systemowym menedżerze haseł zgodnym z Secret Service. Pokazuje stan
  demona, wykrycie telefonu, ostatni wynik sideloadingu, lokalną wersję IPA i
  wynik sprawdzenia najnowszego release; status odświeża się automatycznie.
- Binaria aplikacji są w podkatalogu `bin/`, a `Dopamine.ipa` i
  `Dopamine.version` w katalogu głównym aplikacji:
  `$XDG_DATA_HOME/dopamine-sideload/` (domyślnie `~/.local/share/dopamine-sideload/`).
- Updater codziennie sprawdza najnowszy release `opa334/Dopamine`, waliduje
  archiwum i dostępny SHA-256, po czym atomowo podmienia IPA.
- Instalator domyślnie włącza demona. Przy testowym instalowaniu bez
  uruchamiania monitorowania ustaw `DOPAMINE_START_DAEMON=0`.

## Ograniczenia bieżącej wersji

Projekt wymaga dystrybucji z systemd user session i działającym `usbmuxd`.
Usługi wymagają aktywnej sesji użytkownika; nie włączaj linger, jeśli nie chcesz,
aby działały po wylogowaniu.
