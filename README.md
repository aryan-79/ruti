## Installation

### Unix-like systems

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aryan-79/ruti/releases/latest/download/ruti-installer.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://github.com/aryan-79/ruti/releases/latest/download/ruti-installer.ps1 | iex

```



## Shell Completions

### Bash

Add the following to the end of `~/.bashrc`:

```bash
eval "$(ruti completions bash)"
```

### Zsh

Add the following to the end of `~/.zshrc`:

```zsh
eval "$(ruti completions zsh)"
```

### Fish

Add the following to the end of `~/.config/fish/config.fish`:

```fish
ruti completions fish | source
```

### Elvish

Add the following to the end of `~/.config/elvish/rc.elv`:

```elvish
eval (ruti completions elvish)
```

### Nushell

Add the following to the end of your Nushell configuration (find it by running `$nu.config-path` in Nushell):

```sh
mkdir ($nu.data-dir | path join "vendor/autoload")
ruti completions nushell | save -f ($nu.data-dir | path join "vendor/autoload/ruti.nu")
```

### PowerShell

Add the following to the end of `Microsoft.PowerShell_profile.ps1` (find it by running `$PROFILE`):

```powershell
Invoke-Expression (&ruti completions powershell)
```
