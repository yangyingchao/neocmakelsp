mod config;

use self::config::Config;

use super::Backend;
use crate::ast;
use crate::complete;
use crate::consts::TREESITTER_CMAKE_LANGUAGE;
use crate::filewatcher;
use crate::formatting::format_range;
use crate::formatting::getformat;
use crate::grammar::checkerror;
use crate::jump;
use crate::scansubs;
use crate::scansubs::schedule_scan_all;
use crate::utils::treehelper;
use async_lsp::lsp_types;
use async_lsp::lsp_types::*;
use async_lsp::ErrorCode;
use futures::executor::block_on;
use lsp_types::{
    DidChangeConfigurationParams, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    MarkedString, MessageType, OneOf, ServerCapabilities, ShowMessageParams,
};

use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::Mutex;
use tree_sitter::Parser;

use std::ops::ControlFlow;

use once_cell::sync::Lazy;

use async_lsp::{LanguageClient, LanguageServer, ResponseError};

pub static BUFFERS_CACHE: Lazy<Arc<Mutex<HashMap<lsp_types::Url, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static CLIENT_CAPABILITIES: RwLock<Option<TextDocumentClientCapabilities>> = RwLock::new(None);

fn set_client_text_document(text_document: Option<TextDocumentClientCapabilities>) {
    let mut data = CLIENT_CAPABILITIES.write().unwrap();
    *data = text_document;
}

pub fn get_client_capabilities() -> Option<TextDocumentClientCapabilities> {
    let data = CLIENT_CAPABILITIES.read().unwrap();
    data.clone()
}

pub fn client_support_snippet() -> bool {
    match get_client_capabilities() {
        Some(c) => c
            .completion
            .and_then(|item| item.completion_item)
            .and_then(|item| item.snippet_support)
            .unwrap_or(false),
        _ => false,
    }
}

impl Backend {
    async fn publish_diagnostics(&mut self, uri: Url, context: String, use_cmake_lint: bool) {
        let mut parse = Parser::new();
        parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
        let thetree = parse.parse(&context, None);
        let Some(tree) = thetree else {
            return;
        };
        let gammererror = checkerror(
            Path::new(uri.path()),
            &context,
            tree.root_node(),
            use_cmake_lint,
        );
        if let Some(diagnoses) = gammererror {
            let mut pusheddiagnoses = vec![];
            for (start, end, message, severity) in diagnoses.inner {
                let pointx = lsp_types::Position::new(start.row as u32, start.column as u32);
                let pointy = lsp_types::Position::new(end.row as u32, end.column as u32);
                let range = Range {
                    start: pointx,
                    end: pointy,
                };
                let diagnose = Diagnostic {
                    range,
                    severity,
                    code: None,
                    code_description: None,
                    source: None,
                    message,
                    related_information: None,
                    tags: None,
                    data: None,
                };
                pusheddiagnoses.push(diagnose);
            }
            self.client
                .publish_diagnostics(PublishDiagnosticsParams {
                    uri,
                    diagnostics: pusheddiagnoses,
                    version: Some(1),
                })
                .unwrap();
        } else {
            self.client
                .publish_diagnostics(PublishDiagnosticsParams {
                    uri,
                    diagnostics: vec![],
                    version: None,
                })
                .unwrap();
        }
    }

    fn update_diagnostics(&mut self) {
        let storemap = block_on(BUFFERS_CACHE.lock());
        for (uri, context) in storemap.iter() {
            block_on(self.publish_diagnostics(uri.clone(), context.to_string(), true));
        }
    }
}

impl LanguageServer for Backend {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        initial: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        let initial_config: Config = initial
            .initialization_options
            .and_then(|value| serde_json::from_value(value).unwrap_or(None))
            .unwrap_or_default();

        self.init_info.scan_cmake_in_package = initial_config.is_scan_cmake_in_package();

        if let Some(workspace) = initial.capabilities.workspace {
            if let Some(watch_file) = workspace.did_change_watched_files {
                if let (Some(true), Some(true)) = (
                    watch_file.dynamic_registration,
                    watch_file.relative_pattern_support,
                ) {
                    #[allow(deprecated)]
                    if let Some(ref uri) = initial.root_uri {
                        let path = std::path::Path::new(uri.path())
                            .join("build")
                            .join("CMakeCache.txt");
                        if path.exists() {
                            filewatcher::refresh_error_packages(path);
                        }
                    }
                }
            }
        }

        #[allow(deprecated)]
        if let Some(uri) = initial.root_uri.clone() {
            self.scan_handle
                .replace(schedule_scan_all(uri.path().to_string()));
            self.root_path.replace(uri.path().into());
        }

        set_client_text_document(initial.capabilities.text_document);

        let version: String = env!("CARGO_PKG_VERSION").to_string();

        Box::pin(async move {
            Ok(InitializeResult {
                server_info: Some(ServerInfo {
                    name: "neocmakelsp".to_string(),
                    version: Some(version),
                }),
                capabilities: ServerCapabilities {
                    text_document_sync: Some(TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(TextDocumentSyncKind::FULL),
                            will_save: Some(false),
                            will_save_wait_until: Some(false),
                            save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        },
                    )),
                    completion_provider: Some(CompletionOptions {
                        resolve_provider: Some(false),
                        trigger_characters: None,
                        work_done_progress_options: Default::default(),
                        all_commit_characters: None,
                        completion_item: None,
                    }),
                    document_symbol_provider: Some(OneOf::Left(true)),
                    definition_provider: Some(OneOf::Left(true)),
                    document_formatting_provider: Some(OneOf::Left(true)),
                    document_range_formatting_provider: Some(OneOf::Left(true)),
                    hover_provider: Some(HoverProviderCapability::Simple(true)),
                    workspace: Some(WorkspaceServerCapabilities {
                        workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                            supported: Some(true),
                            change_notifications: Some(OneOf::Left(true)),
                        }),
                        file_operations: None,
                    }),
                    semantic_tokens_provider: None,
                    references_provider: Some(OneOf::Left(true)),
                    ..ServerCapabilities::default()
                },
            })
        })
    }

    fn initialized(&mut self, _: InitializedParams) -> Self::NotifyResult {
        let cachefilechangeparms = DidChangeWatchedFilesRegistrationOptions {
            watchers: vec![
                FileSystemWatcher {
                    glob_pattern: GlobPattern::String("**/CMakeCache.txt".to_string()),
                    kind: Some(lsp_types::WatchKind::all()),
                },
                FileSystemWatcher {
                    glob_pattern: GlobPattern::String("**/CMakeLists.txt".to_string()),
                    kind: Some(lsp_types::WatchKind::Create | lsp_types::WatchKind::Delete),
                },
            ],
        };

        let cmakecache_watcher = Registration {
            id: "CMakeCacheWatcher".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(serde_json::to_value(cachefilechangeparms).unwrap()),
        };

        // TODO: block ???
        let _ = self.client.register_capability(RegistrationParams {
            registrations: vec![cmakecache_watcher],
        });

        self.client
            .show_message(ShowMessageParams {
                typ: MessageType::INFO,
                message: "initialized!".into(),
            })
            .unwrap();
        ControlFlow::Continue(())
    }

    fn did_change_configuration(
        &mut self,
        _: DidChangeConfigurationParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Continue(())
    }

    fn did_change_watched_files(
        &mut self,
        params: DidChangeWatchedFilesParams,
    ) -> ControlFlow<Result<(), async_lsp::Error>> {
        for change in params.changes {
            if let Some("CMakeLists.txt") = change.uri.path().split('/').last() {
                let Some(ref path) = self.root_path else {
                    continue;
                };

                if self.scan_handle.is_some() {
                    let _ = block_on(self.scan_handle.as_mut().unwrap());
                }
                self.scan_handle.replace(scansubs::schedule_scan_all(
                    path.to_str().unwrap().to_string(),
                ));
                continue;
            }
            self.client
                .log_message(LogMessageParams {
                    typ: MessageType::INFO,
                    message: "CMakeCache changed".into(),
                })
                .unwrap();
            if let FileChangeType::DELETED = change.typ {
                filewatcher::clear_error_packages();
            } else {
                let path = change.uri.path();
                filewatcher::refresh_error_packages(path);
            }
        }
        self.update_diagnostics();
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: "watched files have changed!".into(),
            })
            .unwrap();
        ControlFlow::Continue(())
    }

    fn did_open(
        &mut self,
        input: DidOpenTextDocumentParams,
    ) -> ControlFlow<Result<(), async_lsp::Error>> {
        let mut parse = Parser::new();
        parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
        let uri = input.text_document.uri.clone();
        let context = input.text_document.text.clone();
        let mut storemap = block_on(BUFFERS_CACHE.lock());
        storemap.entry(uri.clone()).or_insert(context.clone());
        block_on(self.publish_diagnostics(uri, context, true));
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: "file opened!".into(),
            })
            .unwrap();

        ControlFlow::Continue(())
    }

    fn did_change(
        &mut self,
        input: DidChangeTextDocumentParams,
    ) -> ControlFlow<Result<(), async_lsp::Error>> {
        // create a parse
        let uri = input.text_document.uri.clone();
        let context = input.content_changes[0].text.clone();
        let mut storemap = block_on(BUFFERS_CACHE.lock());
        storemap.insert(uri.clone(), context.clone());
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: format!("{input:?}"),
            })
            .unwrap();
        ControlFlow::Continue(())
    }

    fn did_save(
        &mut self,
        params: DidSaveTextDocumentParams,
    ) -> ControlFlow<Result<(), async_lsp::Error>> {
        let uri = params.text_document.uri;

        let has_root = self.root_path.is_some();
        if has_root {
            block_on(scansubs::scan_dir(uri.path()));
        };

        if let Some(context) = params.text {
            let mut storemap = block_on(BUFFERS_CACHE.lock());
            storemap.insert(uri.clone(), context.clone());
        };

        let storemap = block_on(BUFFERS_CACHE.lock());
        if let Some(context) = storemap.get(&uri) {
            if has_root {
                block_on(complete::update_cache(uri.path(), context));
            }
            block_on(self.publish_diagnostics(uri, context.to_string(), true));
        }

        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: "file saved!".into(),
            })
            .unwrap();

        ControlFlow::Continue(())
    }

    fn hover(
        &mut self,
        params: HoverParams,
    ) -> BoxFuture<'static, Result<Option<Hover>, Self::Error>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let storemap = block_on(BUFFERS_CACHE.lock());
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: "Hovered!".into(),
            })
            .unwrap();

        match storemap.get(&uri) {
            Some(context) => {
                let mut parse = Parser::new();
                parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
                let thetree = parse.parse(context.clone(), None);
                let tree = thetree.unwrap();
                let output = treehelper::get_cmake_doc(position, tree.root_node(), context);
                match output {
                    Some(context) => Box::pin(async move {
                        Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(context)),
                            range: Some(Range {
                                start: position,
                                end: position,
                            }),
                        }))
                    }),
                    None => Box::pin(async move { Ok(None) }),
                }
            }
            None => Box::pin(async move { Ok(None) }),
        }
    }

    fn formatting(
        &mut self,
        input: DocumentFormattingParams,
    ) -> BoxFuture<'static, Result<Option<Vec<TextEdit>>, Self::Error>> {
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: format!("formatting, space is {}", input.options.insert_spaces),
            })
            .unwrap();

        match getformat(Path::new(input.text_document.uri.path())) {
            Ok(result) => Box::pin(async move { Ok(result) }),
            Err(err) => {
                Box::pin(async move { Err(ResponseError::new(ErrorCode::INTERNAL_ERROR, err)) })
            }
        }
    }

    fn range_formatting(
        &mut self,
        input: DocumentRangeFormattingParams,
    ) -> BoxFuture<'static, Result<Option<Vec<TextEdit>>, Self::Error>> {
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: format!("range_formatting, space is {}", input.options.insert_spaces),
            })
            .unwrap();

        let storemap = block_on(BUFFERS_CACHE.lock());
        match storemap.get(&input.text_document.uri) {
            Some(context) => match format_range(context, input.range) {
                Ok(result) => Box::pin(async move { Ok(result) }),
                Err(err) => {
                    Box::pin(async move { Err(ResponseError::new(ErrorCode::INTERNAL_ERROR, err)) })
                }
            },
            _ => Box::pin(async move {
                Err(ResponseError::new(
                    ErrorCode::INTERNAL_ERROR,
                    "file not cached".to_owned(),
                ))
            }),
        }
    }

    fn did_close(
        &mut self,
        params: DidCloseTextDocumentParams,
    ) -> ControlFlow<Result<(), async_lsp::Error>> {
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: format!("file {:?} closed!", params.text_document.uri),
            })
            .unwrap();
        //notify_send("file closed", Type::Info);
        ControlFlow::Continue(())
    }

    fn completion(
        &mut self,
        input: CompletionParams,
    ) -> BoxFuture<'static, Result<Option<CompletionResponse>, Self::Error>> {
        self.client
            .log_message(LogMessageParams {
                typ: MessageType::INFO,
                message: "Complete".into(),
            })
            .unwrap();
        let location = input.text_document_position.position;
        let uri = input.text_document_position.text_document.uri;
        let storemap = BUFFERS_CACHE.lock();
        let urlconent = block_on(storemap).get(&uri).cloned();

        match urlconent {
            Some(context) => {
                let completion = complete::getcomplete(
                    &context,
                    location,
                    &self.client,
                    uri.path(),
                    self.init_info.scan_cmake_in_package,
                );
                Box::pin(async move { Ok(completion) })
            }
            None => Box::pin(async move { Ok(None) }),
        }
    }

    fn references(
        &mut self,
        input: ReferenceParams,
    ) -> BoxFuture<'static, Result<Option<Vec<Location>>, Self::Error>> {
        let uri = input.text_document_position.text_document.uri;

        let location = input.text_document_position.position;
        let storemap = block_on(BUFFERS_CACHE.lock());
        let result = match storemap.get(&uri) {
            Some(context) => {
                let mut parse = Parser::new();
                parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
                //notify_send(context, Type::Error);
                block_on(jump::godef(
                    location,
                    context,
                    uri.path().to_string(),
                    &self.client,
                    false,
                ))
            }
            None => None,
        };
        Box::pin(async move { Ok(result) })
    }

    fn definition(
        &mut self,
        input: GotoDefinitionParams,
    ) -> BoxFuture<'static, Result<Option<GotoDefinitionResponse>, ResponseError>> {
        let uri = input.text_document_position_params.text_document.uri;
        let location = input.text_document_position_params.position;
        let storemap = block_on(BUFFERS_CACHE.lock());
        let result = match storemap.get(&uri) {
            Some(context) => {
                let mut parse = Parser::new();
                parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
                let thetree = parse.parse(context.clone(), None);
                let tree = thetree.unwrap();
                let origin_selection_range =
                    treehelper::get_position_range(location, tree.root_node());

                //notify_send(context, Type::Error);
                block_on(jump::godef(
                    location,
                    context,
                    uri.path().to_string(),
                    &self.client,
                    true,
                ))
                .map(|range| {
                    GotoDefinitionResponse::Link({
                        range
                            .iter()
                            .filter(|input| match origin_selection_range {
                                Some(origin) => origin != input.range,
                                None => true,
                            })
                            .map(|range| LocationLink {
                                origin_selection_range,
                                target_uri: range.uri.clone(),
                                target_range: range.range,
                                target_selection_range: range.range,
                            })
                            .collect()
                    })
                })

                //Ok(None)
            }
            None => None,
        };
        Box::pin(async move { Ok(result) })
    }

    fn document_symbol(
        &mut self,
        input: DocumentSymbolParams,
    ) -> BoxFuture<'static, Result<Option<DocumentSymbolResponse>, Self::Error>> {
        let uri = input.text_document.uri.clone();
        let storemap = block_on(BUFFERS_CACHE.lock());
        let result = match storemap.get(&uri) {
            Some(context) => block_on(ast::getast(&mut self.client, context)),
            None => None,
        };

        Box::pin(async move { Ok(result) })
    }
}
