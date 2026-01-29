# CMake LSP implementation based on Tower and Tree-sitter

Forked from https://github.com/neocmakelsp/neocmakelsp
 

## Differences from upstream

This fork tailors the upstream project for Emacs workflows and adds some performance improvements:

- Parallel initialization and dynamic progress reporting in the language server for faster startup.
- Retained core LSP features (completion, hover, go-to-definition, references, diagnostics, etc.).
- Emacs-centric usage notes, with ready examples for eglot integration.
- Minor changes in source to support improved performance and maintainability (see code changes in `src/languageserver.rs` and related modules).


## Configuration for emacs eglot

``` emacs-lisp
(use-package cmake-ts-mode
  :config
  (add-hook 'cmake-ts-mode-hook
    (defun setup-neocmakelsp ()
      (require 'eglot)
      (add-to-list 'eglot-server-programs `((cmake-ts-mode) . ("neocmakelsp" "stdio")))
      (eglot-ensure)))

  (defun yc/eglot-shutdown-around-a (func &rest args)
    "Ignore errors when shutting down server."
    (ignore-errors (apply func args)))

  (dolist (target (ensure-list 'eglot-shutdown))
    (advice-add target :around #'yc/eglot-shutdown-around-a)))
```

## Requirements

It requires `cmake-lint` & `cmake-format` for linting and formating. Install with pip:

``` shell
uv pip install cmakelang
```

