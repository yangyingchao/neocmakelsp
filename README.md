# CMake LSP implementation based on Tower and Tree-sitter

Forked from https://github.com/neocmakelsp/neocmakelsp
 

## Differences from upstream

This fork tailors the upstream project for Emacs workflows and adds some performance improvements:

- Parallel initialization and dynamic progress reporting in the language server for faster startup.
- Retained core LSP features (completion, hover, go-to-definition, references, diagnostics, etc.).
- Emacs-centric usage notes, with ready examples for eglot integration.
- Minor changes in source to support improved performance and maintainability (see code changes in `src/languageserver.rs` and related modules).


## Configuration for emacs eglot

This fork adjusts server behavior for Eglot:

- **Shutdown**: Server responds to `shutdown` and waits for the client’s `exit` notification instead of calling `exit(0)` in the shutdown handler. Note: tower-lsp has a [known bug](https://github.com/ebkalderon/tower-lsp/issues/399) where the server may not exit after receiving the `exit` notification; Eglot may then kill the process and surface errors, so the shutdown advice below is still recommended.
- **did_close**: Document is removed from the server’s store on close to avoid stale content and unbounded memory.
- **Dynamic registration**: If the client does not support `workspace/didChangeWatchedFiles` registration (e.g. older Eglot), the server logs a warning instead of panicking.
- **Log noise**: Routine events (open/change/save/close) are logged with `tracing::debug` instead of LSP `logMessage`, so the LSP log stays quiet unless you run with `RUST_LOG=neocmakelsp=debug`.
- **Completion**: `trigger_characters: ["("]` so Eglot can trigger completion after `(` for CMake commands.

Recommended config (including shutdown advice, because tower-lsp may not exit on `exit` and Eglot can report errors when it kills the process):

``` emacs-lisp
(use-package cmake-ts-mode
  :config
  (add-hook 'cmake-ts-mode-hook
    (defun setup-neocmakelsp ()
      (require 'eglot)
      (add-to-list 'eglot-server-programs `((cmake-ts-mode) . ("neocmakelsp" "stdio")))
      (eglot-ensure)))

  ;; Recommended: tower-lsp does not exit on exit notification (tower-lsp#399),
  ;; so Eglot may kill the process and report errors; this advice ignores them.
  (defun yc/eglot-shutdown-around-a (func &rest args)
    "Ignore errors when shutting down server."
    (ignore-errors (apply func args)))
  (dolist (target (ensure-list 'eglot-shutdown))
    (advice-add target :around #'yc/eglot-shutdown-around-a)))
```

## Requirements

It requires `cmake-lint` & `cmake-format` for linting and formamting. Install with pip:

``` shell
uv pip install cmakelang
```

