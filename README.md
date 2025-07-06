# Prayer TUI

A simple, minimalistic TUI for displaying prayer times in your terminal.

## Installation (Arch Linux)

1.  Clone the repository:
    ```sh
    git clone https://github.com/your-username/prayer-tui.git
    cd prayer-tui
    ```

2.  Run the installation script:
    ```sh
    chmod +x install.sh
    ./install.sh
    ```

## Configuration

The The application can be configured by editing the `~/.config/prayer-tui/config.toml` file. 

### Options

- `city`: Your city (e.g., "London").
- `country`: Your country (e.g., "UK").
- `method`: The prayer time calculation method. The available options are:
    - `0`: Shia Ithna-Ansari
    - `1`: University of Islamic Sciences, Karachi
    - `2`: Islamic Society of North America
    - `3`: Muslim World League
    - `4`: Umm Al-Qura University, Makkah
    - `5`: Egyptian General Authority of Survey
    - `7`: Institute of Geophysics, University of Tehran
    - `8`: Gulf Region
    - `9`: Kuwait
    - `10`: Qatar
    - `11`: Majlis Ugama Islam Singapura, Singapore
    - `12`: Union Organization islamic de France
    - `13`: Diyanet İşleri Başkanlığı, Turkey
    - `14`: Spiritual Administration of Muslims of Russia
- `madhab`: The madhab (school of thought). The available options are:
    - `0`: Shafi, Maliki, Hanbali
    - `1`: Hanafi

## Uninstallation

To uninstall the application, run the `uninstall.sh` script:

```sh
chmod +x uninstall.sh
./uninstall.sh
```

