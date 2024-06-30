
Upstream repo: https://github.com/Decodetalkers/neocmakelsp

This branch is aimed to make neocmakelsp works better with my emacs workflow.


Main changes:

1. **PURE LSP SERVER**
2. `tower-lsp` --> `async-lsp` for better support for 'shutdown' request
3. remove lots of async tasks: can't understand them good enough for now.
4. no more tcp connection, only STDIN.
5. snippets support (ported to upstream)
6. cmake-lint (ported to upstream)


# CMake LSP implementation based on Tower and Tree-sitter

[![Crates.io](https://img.shields.io/crates/v/neocmakelsp.svg)](https://crates.io/crates/neocmakelsp)

It is a CMake lsp based on tower-lsp and treesitter

## Install

```bash
cargo install neocmakelsp
```

## Setup For neovim

The config of neocmakelsp is in `nvim-lsp-config`, so just follow `nvim-lsp-config` to setup it

```lua
local configs = require("lspconfig.configs")
local nvim_lsp = require("lspconfig")
if not configs.neocmake then
    configs.neocmake = {
        default_config = {
            cmd = { "neocmakelsp", "--stdio" },
            filetypes = { "cmake" },
            root_dir = function(fname)
                return nvim_lsp.util.find_git_ancestor(fname)
            end,
            single_file_support = true,-- suggested
            on_attach = on_attach, -- on_attach is the on_attach function you defined
            init_options = {
                format = {
                    enable = true
                },
                scan_cmake_in_package = true -- default is true
            }
        }
    }
    nvim_lsp.neocmake.setup({})
end
```


## Setup for helix

```toml
[[language]]
name = "cmake"
auto-format = true
language-servers = [{ name = "neocmakelsp" }]

[language-server.neocmakelsp]
command = "neocmakelsp"
args = ["--stdio"]
```

## Setup for emacs

To use neocmakelsp with eglot:

``` emacs-lisp
(use-package cmake-ts-mode
  :config
  (add-hook 'cmake-ts-mode-hook
    (defun setup-neocmakelsp ()
      (require 'eglot)
      (add-to-list 'eglot-server-programs `((cmake-ts-mode) . ("neocmakelsp" "--stdio")))
      (eglot-ensure))))
```

## Help needed

* I do not know if all features will work on mac and windows, so if someone use mac or windows, please help me and send pr for this project.
* I want a comaintainer, who is familiar with mac, windows, and lsp.

## Features

-   watchfile
-   complete
-   symbol_provider
-   On hover
-   Format
-   GO TO Definitation
    -   find_package
    -   include
-   Search cli
-   Get the project struct
-   It is also a cli tool to format
-   Lint

## Lint form 6.0.27

Put a file named `.neocmakelint.toml` under the root of the project.

```toml
command_upcase = "ignore" # "lowercase", "upcase"
```
Then it will check whether the command is all upcase.

### External cmake-lint

When [cmake-lint](https://cmake-format.readthedocs.io/en/latest/cmake-lint.html) is installed, `neocmakelsp` will utilize it to offer linting and code analysis each time the file is saved. This functionality can be enabled or disabled in the `.neocmakelint.toml` file:

```toml
enable_external_cmake_lint = true # true to use external cmake-lint, or false to disable it
```

If `enable_external_cmake_lint` is turned on but `cmake-lint` is not installed, external linting will not report any error message.

### If you want to use watchfile in neovim, set

```lua
capabilities = {
    workspace = {
        didChangeWatchedFiles = {
            dynamicRegistration = true,
        },
    },
}
```

It will check CMakeCache.txt, and get weather the package is exist


### lsp init_options

```lua
init_options = {
    format = {
        enable = true, -- to use lsp format

    },
    scan_cmake_in_package = false, -- it will deeply check the cmake file which found when search cmake packages.
    semantic_token = false,
    -- semantic_token heighlight. if you use treesitter highlight, it is suggested to set with false. it can be used to make better highlight for vscode which only has textmate highlight
}

```

## TODO

-   Undefined function check

## Show

### Search

![Search](./images/search.png)

### symbol

![Symbol](./images/ast.png)

### Complete and symbol support

![Complete](./images/findpackage.png)
![CompleteFindpackage](./images/complete.png)

### OnHover

![onHover](./images/onhover.png)

### GoToDefinition

![Show](https://raw.githubusercontent.com/Decodetalkers/utils/master/cmakelsp/definition.png)
![JumpToFile](./images/Jump.png)

### Tree

![TreeShow](images/tree.png)

### Format cli

_Note: When formatting files, make sure that your .editorconfig file is in your working directory_

```
format the file

Usage: neocmakelsp {format|--format|-F} [OPTIONS] <FormatPath>...

Arguments:
  <FormatPath>...  file or folder to format

Options:
  -o, --override  override
  -h, --help      Print help
```

It will read .editorconfig file to format files, just set like

```ini
[CMakeLists.txt]
indent_style = space
indent_size = 4
```

#### Note

The format do the min things, just do `trim` and place the first line to the right place by the indent you set, this means

```cmake
function(A)

        set(A
        B
            C
        )

    endfunction()
```

it will just become

```cmake

function(A)

    set(A
        B
            C
        )

endfunction()
```

It just remove the space in the end, replace `\t` at the begin of each line to ` `, if set `indent_size` to space, and format the first line to right place. It does little, but I think it is enough.
