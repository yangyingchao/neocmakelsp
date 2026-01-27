# CMake LSP implementation based on Tower and Tree-sitter

Forked from https://github.com/neocmakelsp/neocmakelsp

Make it just work for my emacs workflow:

1. Remove configuration file and other non-LSP functionalities.
2. Always use cmake-lint to linting, and cmake-format for format (inside emacs, not via LSP)
3. add "--debug" to make server pause after start (so can be attached with debugger).
4. Tested on Linux ONLY.

## Emacs

To use `neocmakelsp` with eglot:

``` emacs-lisp
(use-package cmake-ts-mode
  :config
  (add-hook 'cmake-ts-mode-hook
    (defun setup-neocmakelsp ()
      (require 'eglot)
      (add-to-list 'eglot-server-programs `((cmake-ts-mode) . ("neocmakelsp" "stdio")))
      (eglot-ensure))))
```

