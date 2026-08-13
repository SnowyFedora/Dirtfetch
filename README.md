# dirtfetch 🦀

fastfetch-style system fetch на Rust: неоновая рамка с градиентом, кастомные
ASCII-логотипы, JSONC-конфиг и куча автоопределений.

## Фишки

- 🖼 **56 ASCII-логотипов** в комплекте (папка `logos/`), свои — кидай в
  `~/.config/dirtfetch/logos/<name>.txt`
- 🎨 фирменные цвета дистрибутивов + маркеры `${c1}/${c2}` из neofetch-артов
- 🌈 неоновая рамка с градиентом, `--neon`, пасхалки `--trad` и `--homo`
- ⚙️ JSONC-конфиг как в fastfetch: комментарии, trailing commas, порядок модулей
- 📦 все пакетные менеджеры: pacman, dpkg, rpm, xbps, apk, flatpak, snap
- 💽 все реальные диски (без `/boot`), WM/DE, GPU, батарея, разрешение, IP

## Установка с GitHub

```bash
# 1. клонируем репозиторий
git clone https://github.com/SnowyFedora/dirtfetch.git
cd dirtfetch

# 2. ставим бинарник в ~/.cargo/bin
cargo install --path .

# 3. копируем лого из репы в конфиг
mkdir -p ~/.config/dirtfetch/logos
cp -r logos/* ~/.config/dirtfetch/logos/

# 4. профит
dirtfetch
```

Или совсем без клона:

```bash
cargo install --git https://github.com/SnowyFedora/dirtfetch.git
```

(в этом случае лого потом всё равно докопируй из клона, см. шаг 3)

Обновление:

```bash
cd dirtfetch && git pull && cargo install --path . --force
```

> Если `dirtfetch: command not found` — добавь `~/.cargo/bin` в PATH:
> `echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc`

## Установка из исходников (локально)

```bash
cargo install --path .
mkdir -p ~/.config/dirtfetch/logos
cp -r logos/* ~/.config/dirtfetch/logos/
```

## Флаги

| Флаг | Что делает |
|------|------------|
| `-l / --logo NAME` | форсировать логотип (`dirtfetch -l fedora`) |
| `--list-logos` | список доступных лого |
| `--no-logo` | без логотипа |
| `--no-color` | без цвета (или `NO_COLOR=1`) |
| `--no-frame` | отключить неоновую рамку |
| `--neon` | градиент логотипа cyan → magenta |
| `--trad` | логотип в цвета флага натуралов |
| `--homo` | логотип в цвета прайд-флага |
| `--hide a,b,c` | скрыть модули (`--hide swap,battery`) |
| `--gen-config` | создать пример `config.json` |
| `-help` / `--help` | справка по флагам |

Работает и в стиле одного дефиса: `-help`, `-trad`, `-neon`, `-logo`…

## Конфиг

`dirtfetch --gen-config` → `~/.config/dirtfetch/config.json`:

```jsonc
{
    "logo": { "type": "auto" },          // auto | off | имя
    "display": {
        "separator": ":",
        "palette": true,
        "neon": false,
        "frame": true
        // "color": "magenta"
    },
    "modules": [ "title", "separator", "os", "cpu", "memory", "disk" ]
    // объекты тоже можно: { "type": "disk", "mount": "/home", "label": "Home" }
}
```

## Свои логотипы

Файл `~/.config/dirtfetch/logos/<name>.txt`:

```
#color=cyan            (опционально) акцент
#colors=blue,white     (опционально) палитра для ${c1}/${c2}
                   -`
                  .o+`
                 ...сам арт...
```

Приоритет: твой txt → встроенный арт. Имя файла = имя для `-l`.

## Зависимости

Только crates: `clap`, `dirs`, `serde`, `serde_json`, `sysinfo`.
Системных зависимостей нет (всё через `/proc`, `/sys`, env и `lspci`/`xrandr`,
если они есть).
