//use lsp_types::CompletionItem;
use super::Location;
use crate::utils;
use async_lsp::lsp_types;
use async_lsp::ClientSocket;
use lsp_types::Url;
pub(super) async fn cmpfindpackage(input: String, _client: &ClientSocket) -> Option<Vec<Location>> {
    utils::CMAKE_PACKAGES_WITHKEY.get(&input).map(|context| {
        context
            .tojump
            .iter()
            .map(|apath| Location {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                },
                uri: Url::from_file_path(apath).unwrap(),
            })
            .collect()
    })
}
